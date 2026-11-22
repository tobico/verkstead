//! One of this device's Agent Profiles as a member is handed its **account**:
//! the files a Built Root is made out of, and nothing else of it (ADR-0020,
//! *Shared Profiles*).
//!
//! **The answering half of the account mirror.** A device launching under a
//! Profile bound here and not there keeps a mirror of that account under its own
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
//! **And only the Profiles bound here.** A mirror is written from a binding, so
//! a Profile this device itself runs from a copy of is an account bound on a
//! third machine and this device has no files of it to hand over: it answers as
//! it answers a Profile that is not there at all, and the asking device is one
//! hop from the machine the account is on either way.
//!
//! **And one thing comes back.** A login is the one file of a root a session
//! genuinely changes — the harness refreshes its OAuth pair as it works — so an
//! account lent out and never written back would be an account signing itself out
//! a session at a time. As a session run from a copy ends, that device puts what
//! it left into the account here, on [`LOGIN`] beside the read. Nothing else
//! comes back: everything else in a root is either Verkstead's own, written fresh
//! each launch, or the human's own and never a session's to change.

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use verkstead_render::{AccountFile, AccountLogin, AccountLoginTaken, AccountSignedOut};

use crate::{AppState, store};

/// Where a member asks after the account of a Profile bound on this device, by
/// its uuid — which is the one it has on every device.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace and is drawn by a
/// browser. The same shape the news, the renewal and the Repos beside it take.
pub const ACCOUNT: &str = "/api/peer/v1/profiles/{profile}/account";

/// And where a device the Profile is not bound on puts the **login** its session
/// left, for the same Profile by the same uuid.
///
/// Under the account rather than beside it, because that is what it is one file
/// of: the read hands over what a root is made from and this hands one of those
/// back, and the two are the two directions of one thing.
pub const LOGIN: &str = "/api/peer/v1/profiles/{profile}/account/login";

/// And where that device says a session there found the account **signed
/// out**, so that the card here offers the login — see [`signed_out`].
///
/// Under the account for [`LOGIN`]'s reason: it is about the login this device
/// handed over, and nothing else.
pub const SIGNED_OUT: &str = "/api/peer/v1/profiles/{profile}/account/signed-out";

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
/// session run from a copy is the one the next such session has to be given, so
/// nothing is held: the answer is the account as it is now, and a mirror is
/// written from it before each launch.
///
/// **A Profile that is not bound here is `404`**, and so is one that is not
/// there at all — a record removed between the asking device's last pull and
/// this call, which is the ordinary way this answers nothing.
///
/// The reads go in one blocking task: they are a handful of small files, and a
/// launch asks once.
pub(crate) async fn held(State(state): State<AppState>, Path(profile): Path<String>) -> Response {
    let held = match store::load_profile(&state.pool, &profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile = %profile,
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

    // A Profile bound here and nothing it runs from a copy of: the files of a
    // Profile bound on a third machine are that machine's to hand over, and a
    // device that relayed them on would be two hops from the account it is
    // about to launch under.
    let Some(account) = held
        .filter(|held| held.unbound.is_none())
        .map(|held| held.account)
    else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no binding of an Agent Profile under that id\n",
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
                profile = %profile,
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

/// `POST /api/peer/v1/profiles/{profile}/account/login` — the login a session
/// run from a copy left, written into that Profile's account.
///
/// **The answering half of the write-back**, and the one direction anything of an
/// account travels in on its way *back*: a harness refreshes its OAuth pair as it
/// works, so a login lent out and never returned is an account signing itself out
/// a session at a time. Everything else a root holds is Verkstead's own or the
/// human's own and never comes back — see
/// [`crate::mirroring::account::Lending::written_home`], which is the end that
/// reads what the session left and puts it here.
///
/// **Written in place over the account's own file**, which is how a local session's
/// ending writes one: whatever else is a name for that file — a session running
/// here right now has the login hard-linked or bound into its root — is a name for
/// what arrived, rather than for a file left behind by a rename. Owner-only,
/// because it is a login.
///
/// **Written only over the login it names**, and nothing is merged. The login
/// arrives naming the one the sending device last knew this account to hold — see
/// [`AccountLogin::over`] — and a login here that is no longer that one was made or
/// refreshed here since, which is newer than anything that session had: of two
/// machines refreshing one login only the first holds a live one, and a session
/// that signed out on a token rotated away under it would otherwise put that dead
/// token over the live one here — and every session fetching from here after it
/// would come up signed out (ADR-0020, *Shared Profiles*). Such a login is kept,
/// and the answer says so: see [`AccountLoginTaken`]. Nothing is held between the
/// read and the write — see [`crate::mirroring::account::goes_over`].
///
/// **A Profile that is not bound here is `404`**, as the read beside it answers
/// one: a Profile this device runs from a copy of is an account bound on a third
/// machine, and a record that has gone is a Profile removed while a session ran
/// under it.
pub(crate) async fn written(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(sent): Json<AccountLogin>,
) -> Response {
    let held = match store::load_profile(&state.pool, &profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile = %profile,
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

    let Some(held) = held.filter(|held| held.unbound.is_none()) else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no binding of an Agent Profile under that id\n",
        )
            .into_response();
    };

    let login = crate::sandbox::root::login_at(&held.account);

    match tokio::task::spawn_blocking({
        let login = login.clone();

        move || {
            if !crate::mirroring::account::goes_over(&login, &sent) {
                return Ok(false);
            }

            crate::mirroring::account::into_the_account(&login, &sent.text).map(|()| true)
        }
    })
    .await
    {
        Ok(Ok(false)) => {
            tracing::info!(
                profile = %profile,
                "a member's session sent back a login of this device's account that has been \
                 made or refreshed here since, so the account keeps its own",
            );

            Json(AccountLoginTaken { written: false }).into_response()
        }

        Ok(Ok(true)) => {
            tracing::debug!(
                profile = %profile,
                "the login a member's session left was written into this device's account",
            );

            // An account that had none holds one now, which its binding says
            // for every device that would fetch it — see
            // [`crate::profiles::looked`].
            crate::profiles::looked(&state, std::slice::from_ref(&held)).await;

            Json(AccountLoginTaken { written: true }).into_response()
        }

        Ok(Err(why)) => {
            tracing::error!(
                error = ?why,
                profile = %profile,
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
                profile = %profile,
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

/// `POST /api/peer/v1/profiles/{profile}/account/signed-out` — a session run
/// from a copy of this account, on the calling device, found it signed out.
///
/// **This device is where the login is made, and nothing it can look at says
/// it is needed.** The login file is still there, and the harness asked about it
/// may call it fine — so the card here would go on reading *Logged in*, with no
/// Log in to press, while every session elsewhere comes up signed out. So the
/// device that saw it says so, and the Profile is marked here the way a press
/// here that found it signed out marks it: see [`crate::logins`].
///
/// **Only where the login here is still the one that signed out.** The call
/// names the login that came down by its hash — see [`AccountSignedOut`] — and
/// a login made here since is the fix having already arrived: marking that one
/// would offer the human a login they have just done. Either way the answer is
/// `204`, there being nothing for the caller to do differently.
///
/// **A Profile that is not bound here is `404`**, as the two beside it answer
/// one.
pub(crate) async fn signed_out(
    State(state): State<AppState>,
    Path(profile): Path<String>,
    Json(sent): Json<AccountSignedOut>,
) -> Response {
    let held = match store::load_profile(&state.pool, &profile).await {
        Ok(held) => held,

        Err(why) => {
            tracing::error!(
                error = ?why,
                profile = %profile,
                "a member said one of this device's accounts signed out and the Profile could \
                 not be read",
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own Profile\n",
            )
                .into_response();
        }
    };

    let Some(held) = held.filter(|held| held.unbound.is_none()) else {
        return (
            StatusCode::NOT_FOUND,
            "this device holds no binding of an Agent Profile under that id\n",
        )
            .into_response();
    };

    let login = crate::sandbox::root::login_at(&held.account);
    let here = tokio::task::spawn_blocking(move || crate::mirroring::account::login_named(&login))
        .await
        .unwrap_or(None);

    if here == sent.login {
        tracing::info!(
            profile = %profile,
            "a member found this device's account signed out, so its card offers the login",
        );

        state.logins.signed_out_elsewhere(&state, &profile);
    } else {
        tracing::info!(
            profile = %profile,
            "a member found this device's account signed out on a login that has been \
             replaced here since, so nothing is marked",
        );
    }

    StatusCode::NO_CONTENT.into_response()
}

/// The routes, over the state the workbench answers out of.
///
/// **Built here and mounted by [`super::workbench::served`]**, for the reason the
/// Repos beside it are: they answer out of this device's own Profiles, and a
/// router over a second state would be answering for a second device's accounts.
pub(crate) fn route() -> Router<AppState> {
    Router::new()
        .route(ACCOUNT, get(held))
        .route(
            LOGIN,
            post(written).layer(DefaultBodyLimit::max(MOST_A_LOGIN_IS)),
        )
        .route(
            SIGNED_OUT,
            post(signed_out).layer(DefaultBodyLimit::max(MOST_A_LOGIN_IS)),
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
        for path in [ACCOUNT, LOGIN, SIGNED_OUT] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And both are outside the prefixes this device keeps to itself, which
    /// is what they have to be: a route served on the one namespace a member is
    /// never served would be a route nothing could ever reach.
    #[test]
    fn the_account_is_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [ACCOUNT, LOGIN, SIGNED_OUT] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }

    /// And each names the Profile by its uuid, which is the one it has on every
    /// device, the one that answers among them.
    #[test]
    fn the_account_names_the_profile() {
        for path in [ACCOUNT, LOGIN, SIGNED_OUT] {
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
