//! One of this device's Agent Profiles as a member is handed its **account**:
//! the files a Built Root is made out of, and nothing else of it (ADR-0020,
//! *Shared Profiles*).
//!
//! **The answering half of the account mirror.** A device launching under a
//! Profile whose account lives here keeps a mirror of that account under its own
//! Data Directory and builds the session's root out of it exactly as it builds
//! one out of a local account — so this is what the mirror is written from, and
//! [`crate::mirroring::account`] is the end that asks and writes it down.
//!
//! **A member's own call rather than the viewer's**, so it is here beside
//! [`super::news`] and [`super::repos`] rather than in [`crate::ui::routes`]: it
//! carries the account's *files* rather than a view of them, no page draws them,
//! and the one caller is a device before a launch. The shape the news a member
//! tells its members already has.
//!
//! **Gated to members like everything else on this listener.** What it hands
//! over is a login: [`super::router`] puts [`super::members_only`] over it, and a
//! caller whose certificate this device holds no membership for never reaches it.
//!
//! **The allowlist and nothing beside it** — see
//! [`crate::sandbox::root::mirrored`], which is the one list of what a root is
//! made from and is what this answers out of. No plugins, hooks, rules, skills,
//! global instructions file, history, or any other repository's transcripts; and
//! the memory store is not on it, being synced entry by entry on its own terms.
//!
//! **And only this device's own Profiles.** A mirror is written from a member's
//! own rows, so a row that is itself a mirror is an account on a third machine
//! and this device has no files of it to hand over: it answers as it answers a
//! Profile that is not there at all, and the asking device is one hop from the
//! machine the account is on either way.

use axum::Json;
use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use verkstead_render::AccountFile;

use crate::{AppState, store};

/// Where a member asks after the account of one of this device's Profiles, by
/// the id it has **here** — which is the id the mirror row records for it.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser. The same shape the news, the renewal and the Repos beside it take.
pub const ACCOUNT: &str = "/api/peer/v1/profiles/{profile}/account";

/// `GET /api/peer/v1/profiles/{profile}/account` — every file a Built Root is
/// made out of for that Profile's account, each with what is in it or with
/// nothing where the account has no such file.
///
/// **Read at the moment it is asked for.** A login refreshed here since the last
/// session away is the one the next session away has to be given, so nothing is
/// held: the answer is the account as it is now, and a mirror is written from it
/// before each launch.
///
/// **A Profile that is not one of this device's own is `404`**, and so is one
/// that is not there at all — a row removed between a mirror's last refresh and
/// this call, which is the ordinary way this answers nothing.
///
/// The reads go in one blocking task: they are a handful of small files, and a
/// launch asks once.
pub(crate) async fn held(State(state): State<AppState>, Path(profile): Path<i64>) -> Response {
    let held = match store::load_profile(&state.pool, profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "a member asked after one of this device's accounts and the Profile could not \
                 be read",
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own Profile\n",
            )
                .into_response();
        }
    };

    // Its own rows and nothing it is itself mirroring: the files of a Profile at
    // home on a third machine are that machine's to hand over, and a device that
    // relayed them on would be two hops from the account it is about to launch
    // under.
    let Some(account) = held
        .filter(|held| held.mirror.is_none())
        .map(|held| held.account)
    else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no Agent Profile of its own under that id\n",
        )
            .into_response();
    };

    match tokio::task::spawn_blocking(move || crate::sandbox::root::mirrored(&account)).await {
        Ok(files) => Json(
            files
                .into_iter()
                .map(|(inside, text)| AccountFile {
                    // Said with forward slashes whichever machine composed it:
                    // the paths are the harness's own relative names, and the
                    // device that writes them down joins them onto a home of its
                    // own. A Windows account's would otherwise arrive spelled in
                    // a way a Unix device could not take apart.
                    inside: inside.to_string_lossy().replace('\\', "/"),

                    // And a file no harness wrote reads as none rather than as
                    // bytes a mirror would put where a login goes — see
                    // [`AccountFile::text`].
                    text: text.and_then(|bytes| String::from_utf8(bytes).ok()),
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "the account a member asked after could not be read off the disk",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own account\n",
            )
                .into_response()
        }
    }
}

/// The route, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, for the reason the
/// Repos beside it are: it answers out of this device's own Profiles, and a
/// router over a second state would be answering for a second device's accounts.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(ACCOUNT, get(held))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// It is a member's call, so it goes where a member's calls go rather than
    /// through any of the three the gate stands aside for: what it hands over is
    /// a login.
    #[test]
    fn the_account_is_not_one_of_the_un_gated_three() {
        assert_ne!(ACCOUNT, super::super::IDENTITY);
        assert_ne!(ACCOUNT, super::super::joining::JOIN);
        assert_ne!(ACCOUNT, super::super::exchange::SETTLED);
    }

    /// And it is outside the three prefixes this device keeps to itself, which is
    /// what it has to be: a reading served on the one namespace a member is never
    /// served would be a route nothing could ever reach.
    #[test]
    fn the_account_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            assert!(
                !ACCOUNT.starts_with(prefix),
                "the account would be held back with {prefix}/",
            );
        }
    }

    /// And it names the Profile by the id it has on the device that answers,
    /// which is what a mirror row records for it there.
    #[test]
    fn the_account_names_the_profile() {
        assert!(
            ACCOUNT.contains("{profile}"),
            "the account path names the Profile it is about: {ACCOUNT}",
        );
    }
}
