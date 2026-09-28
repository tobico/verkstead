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
//!
//! **And one thing comes back.** A login is the one file of a root a session
//! genuinely changes — the harness refreshes its OAuth pair as it works — so an
//! account lent out and never written back would be an account signing itself out
//! a session at a time. As a session away from home ends, that device puts what it
//! left into the account here, on [`LOGIN`] beside the read. Nothing else comes
//! home: everything else in a root is either Verkstead's own, written fresh each
//! launch, or the human's own and never a session's to change.

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use verkstead_render::{AccountFile, AccountLogin};

use crate::{AppState, store};

/// Where a member asks after the account of one of this device's Profiles, by
/// the id it has **here** — which is the id the mirror row records for it.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser. The same shape the news, the renewal and the Repos beside it take.
pub const ACCOUNT: &str = "/api/peer/v1/profiles/{profile}/account";

/// And where a device away from home puts the **login** its session left, for the
/// same Profile by the same id.
///
/// Under the account rather than beside it, because that is what it is one file
/// of: the read hands over what a root is made from and this hands one of those
/// back, and the two are the two directions of one thing.
pub const LOGIN: &str = "/api/peer/v1/profiles/{profile}/account/login";

/// The most a login sent back here may be before the call is refused rather than
/// read: **sixty-four kilobytes**.
///
/// An OAuth pair is a few hundred bytes, and what a bound is for here is what every
/// bound across a link is for — a machine on the far end that writes without
/// stopping. The mirror's own bound is the other direction's, for its reason — see
/// [`crate::mirroring::account`].
const MOST_A_LOGIN_IS: usize = 64 * 1024;

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

/// `POST /api/peer/v1/profiles/{profile}/account/login` — the login a session away
/// from home left, written into that Profile's account.
///
/// **The answering half of the write-back**, and the one direction anything of an
/// account travels in on its way *back*: a harness refreshes its OAuth pair as it
/// works, so a login lent out and never returned is an account signing itself out
/// a session at a time. Everything else a root holds is Verkstead's own or the
/// human's own and never comes home — see
/// [`crate::mirroring::account::Lending::written_home`], which is the end that
/// reads what the session left and puts it here.
///
/// **Written in place over the account's own file**, which is how a local session's
/// ending writes one: whatever else is a name for that file — a session running
/// here right now has the login hard-linked or bound into its root — is a name for
/// what arrived, rather than for a file left behind by a rename. Owner-only,
/// because it is a login.
///
/// **Last write wins, and nothing is merged.** Two devices refreshing one login may
/// sign one of them out; that is accepted rather than locked against, and the write
/// that arrives later is the one the account keeps (ADR-0020, *Shared Profiles*).
/// So nothing here reads what is already at the path, and nothing is held.
///
/// **A Profile that is not one of this device's own is `404`**, as the read beside
/// it answers one: a row that is itself a mirror is an account on a third machine,
/// and a row that has gone is a Profile removed while a session ran under it.
pub(crate) async fn written(
    State(state): State<AppState>,
    Path(profile): Path<i64>,
    Json(sent): Json<AccountLogin>,
) -> Response {
    let held = match store::load_profile(&state.pool, profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "a member sent back the login of one of this device's accounts and the Profile \
                 could not be read",
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own Profile\n",
            )
                .into_response();
        }
    };

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

    let login = crate::sandbox::root::login_at(&account);

    match tokio::task::spawn_blocking({
        let login = login.clone();

        move || crate::mirroring::account::into_the_account(&login, &sent.text)
    })
    .await
    {
        Ok(Ok(())) => {
            tracing::debug!(
                profile,
                "the login a member's session left was written into this device's account",
            );

            StatusCode::NO_CONTENT.into_response()
        }

        Ok(Err(why)) => {
            tracing::error!(
                error = ?why,
                profile,
                login = %login.display(),
                "the login a member's session left could not be written into this device's \
                 account, so that account may read as signed out here",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the login into its own account\n",
            )
                .into_response()
        }

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile,
                "writing the login a member's session left ended badly",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not write the login into its own account\n",
            )
                .into_response()
        }
    }
}

/// The routes, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, for the reason the
/// Repos beside it are: they answer out of this device's own Profiles, and a
/// router over a second state would be answering for a second device's accounts.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(ACCOUNT, get(held)).route(
        LOGIN,
        post(written).layer(DefaultBodyLimit::max(MOST_A_LOGIN_IS)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both are a member's calls, so they go where a member's calls go rather than
    /// through any of the three the gate stands aside for: what one hands over is
    /// a login, and what the other takes is one.
    #[test]
    fn the_account_is_not_one_of_the_un_gated_three() {
        for path in [ACCOUNT, LOGIN] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And both are outside the three prefixes this device keeps to itself, which
    /// is what they have to be: a route served on the one namespace a member is
    /// never served would be a route nothing could ever reach.
    #[test]
    fn the_account_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [ACCOUNT, LOGIN] {
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
    fn the_account_names_the_profile() {
        for path in [ACCOUNT, LOGIN] {
            assert!(
                path.contains("{profile}"),
                "the path names the Profile it is about: {path}",
            );
        }
    }

    /// And the login is one file of the account rather than a route beside it, the
    /// two being the two directions of one thing.
    #[test]
    fn the_login_is_under_the_account() {
        assert_eq!(LOGIN, format!("{ACCOUNT}/login"));
    }
}
