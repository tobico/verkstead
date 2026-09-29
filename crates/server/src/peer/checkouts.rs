//! What this device already holds of a repository, and the checkout of a
//! Conversation a member is moving onto it (ADR-0020, *Transfer*).
//!
//! **The receiving half of a move's git leg.** The device holding the work packs
//! the branch as a bundle and reads the Worktree's working changes; this is
//! where both land — see [`crate::transfers::checkouts`], which is the end that
//! asks and sends.
//!
//! **Two calls, and the reading comes first.** A bundle packed against nothing
//! carries a repository's whole history down a link for work that is one commit
//! deep, so the sending device asks what this one already has and packs against
//! exactly that. The tips are commits rather than refs: what a prerequisite is
//! is a commit, and which branch of this machine's own happens to be standing on
//! one is nobody else's business.
//!
//! **This device names its own path.** Nothing that arrives is ever joined onto
//! a directory here: the Worktree is cut under *this* machine's Data Directory
//! at the path this machine would have chosen for that work, the way
//! [`super::memory`] names its own parts. What a member sends is a branch, some
//! commits and some bytes.
//!
//! **And this machine's own ignore rules have the last word.** The untracked
//! files that travelled were picked out by the sending machine's rules, and what
//! a checkout here shows is this one's — a file this device would hide is one it
//! builds for itself rather than one it is given.
//!
//! **Gated to members like everything else on this listener.** Writing a branch
//! into somebody's repository is not a stranger's press: [`super::router`] puts
//! [`super::members_only`] over this, and a caller whose certificate this device
//! holds no membership for never reaches it.

use std::path::{Component, Path, PathBuf};

use axum::Json;
use axum::Router;
use axum::extract::{DefaultBodyLimit, Path as At, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use verkstead_render::{CheckoutAcross, TipsThere, UntrackedFile};

use crate::repos::{self, git};
use crate::transfers::checkouts::MOST_A_CHECKOUT_IS;
use crate::{AppState, store};

/// Where a member asks what this device already holds of one of its Repos, by
/// the id it has **here** — which is the id the match settled a moment ago.
///
/// **Not under `/api/ui/`**, which is the viewer's namespace: no browser draws a
/// list of commit ids. Beside the Repos it is a reading of — see
/// [`super::repos`] — and named for what it answers rather than for what the
/// caller will do with it: a tip is a tip whether the asker is packing a bundle
/// or counting.
pub const TIPS: &str = "/api/peer/v1/repos/{repo}/tips";

/// And where the checkout itself lands, against the Conversation this device
/// numbered when the row arrived.
///
/// **A leg of its own rather than more of the record's**, for the record's own
/// reason: a Worktree hangs off a Conversation, and there has to be one here for
/// it to hang from.
pub const ONE_TRANSFER_CHECKOUT: &str = "/api/peer/v1/transfers/{id}/checkout";

/// The most tips this device answers with: **one thousand and twenty-four**.
///
/// A repository worked in for years holds thousands of refs, nearly all of them
/// at commits some other tip already reaches, and the ones worth packing against
/// are the ones somebody has touched — so they go most recently moved first and
/// the tail is left off. What the bound costs is a bundle carrying a little more
/// history than it strictly had to; what no bound would cost is a reading that
/// grows with somebody's branch list.
pub(crate) const MOST_THE_TIPS_ARE: usize = 1024;

/// That path with the Repo filled in, for the device doing the asking.
pub(crate) fn tips_of(repo: i64) -> String {
    TIPS.replace("{repo}", &repo.to_string())
}

/// `GET /api/peer/v1/repos/{repo}/tips` — the commits this device holds at the
/// refs of that Repo.
///
/// **A Repo that is not on this registry answers with nothing rather than
/// refusing.** The caller's next move either way is to pack against what came
/// back, and a registry that no longer holds the repository the match settled on
/// is a move that is about to be refused by the leg that writes — where it can
/// be said in a sentence about the work rather than about a reading.
///
/// The git read goes in one blocking task: it is one `for-each-ref` over a
/// directory, asked once before a move rather than on every page.
pub(crate) async fn held(State(state): State<AppState>, At(repo): At<i64>) -> Response {
    let path = match store::load_repo(&state.pool, repo).await {
        Ok(Some(repo)) => repo.path,

        Ok(None) => return Json(TipsThere { tips: Vec::new() }).into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                repo,
                "a member asked what this device holds of a Repo and the registry could not be read",
            );

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own Repos\n",
            )
                .into_response();
        }
    };

    match tokio::task::spawn_blocking(move || tips(&path)).await {
        Ok(tips) => Json(TipsThere { tips }).into_response(),

        Err(why) => {
            tracing::error!(
                error = ?why,
                repo,
                "reading what this device holds of a Repo ended badly",
            );

            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read its own repository\n",
            )
                .into_response()
        }
    }
}

/// The commits at this repository's refs, most recently moved first and deduped.
///
/// A repository git will not answer about holds nothing, which is the same
/// answer as a repository with no refs: either way the bundle is packed against
/// nothing and everything crosses, which is slow rather than wrong.
fn tips(repo: &Path) -> Vec<String> {
    let listed = git(
        repo,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            &format!("--count={MOST_THE_TIPS_ARE}"),
            "--format=%(objectname)",
        ],
    )
    .unwrap_or_default();

    let mut seen = std::collections::HashSet::new();

    listed
        .lines()
        .map(str::trim)
        .filter(|tip| !tip.is_empty())
        .filter(|tip| seen.insert(tip.to_string()))
        .map(str::to_owned)
        .collect()
}

/// `POST /api/peer/v1/transfers/{id}/checkout` — the branch fetched out of the
/// bundle, the Worktree cut, and the working changes put back into it.
///
/// **Everything or nothing.** A checkout that was cut and then could not be
/// written into is taken back before this answers, because the device that sent
/// it reads a refusal here as *the move did not happen* and sweeps the record —
/// and a directory left behind under a Conversation that has been swept is a
/// checkout nobody can account for.
pub(crate) async fn cut(
    State(state): State<AppState>,
    At(id): At<i64>,
    Json(arriving): Json<CheckoutAcross>,
) -> Response {
    let conversation = match store::load_conversation(&state.pool, id).await {
        Ok(Some(conversation)) => conversation,

        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                "no Conversation on this device has that id\n",
            )
                .into_response();
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "reading a Conversation a member is moving a checkout onto failed");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the Conversation could not be read on this device\n",
            )
                .into_response();
        }
    };

    let (carried, alongside) = match unpacked(&arriving) {
        Ok(carried) => carried,
        Err(why) => return (StatusCode::BAD_REQUEST, format!("{why}\n")).into_response(),
    };

    // The Companions as *this* device's record has them, which is where the mode
    // and the base's name are read from: they arrived with the record a leg
    // earlier, under Repos of this registry.
    let companions = match store::companions(&state.pool, id).await {
        Ok(companions) => companions,

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "reading the companions of a Conversation a member is moving here failed");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "this device could not read the Conversation's companions\n",
            )
                .into_response();
        }
    };

    // The path this device would have cut that work in, named off its own Data
    // Directory and its own Repo's name — never off anything that arrived.
    let worktree = crate::worktrees::worktree_path(
        &state.data_dir,
        id,
        &conversation.repo.name,
        &arriving.branch,
    );

    let mut making = vec![Making {
        repo: conversation.repo.path.clone(),
        path: worktree.clone(),
        branch: Some(arriving.branch.clone()),
        companion: None,
        base_commit: None,
        carried,
    }];

    if let Err(why) = beside(&state, id, &companions, alongside, &mut making) {
        tracing::error!(
            conversation_id = id,
            branch = %arriving.branch,
            "a companion of a Conversation a member is moving here could not be placed: {why}",
        );

        return (StatusCode::BAD_REQUEST, format!("{why}\n")).into_response();
    }

    // Blocking throughout: a fetch, a checkout of a whole tree and a walk over
    // the untracked files, none of which belongs on the runtime's threads — and
    // one task for every checkout, because they are made or unmade together. The
    // plan goes in and comes back rather than being copied into it: what it holds
    // is the bundles and the patches, which is as much as sixty-four megabytes.
    let made = tokio::task::spawn_blocking(move || {
        let outcome = landed(&making);

        (making, outcome)
    })
    .await;

    let making = match made {
        Ok((making, Ok(()))) => making,

        Ok((_, Err(why))) => {
            tracing::error!(
                conversation_id = id,
                branch = %arriving.branch,
                "the checkout of a Conversation a member is moving here could not be made: {why}",
            );

            return (StatusCode::BAD_REQUEST, format!("{why}\n")).into_response();
        }

        Err(why) => {
            tracing::error!(error = ?why, conversation_id = id, "making the checkout of an arriving Conversation ended badly");

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "the checkout could not be made on this device\n",
            )
                .into_response();
        }
    };

    // And where they went, so that everything downstream — a session's binds, the
    // Code pane, the sweep that removes them — is looking at the directories this
    // device just made. With what each was cut from beside it, which is what the
    // commit sweep here leaves out of this Conversation's own work.
    let written = store::arrived_checkout(
        &state.pool,
        id,
        &worktree,
        arriving.base_commit.as_deref(),
        arriving.base_ref.as_deref(),
        &recorded(&making),
    )
    .await;

    if let Err(why) = written {
        tracing::error!(error = ?why, conversation_id = id, "the checkout of an arriving Conversation could not be written down");

        let _ = tokio::task::spawn_blocking(move || taken_back(&making)).await;

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "the checkout could not be written down on this device\n",
        )
            .into_response();
    }

    tracing::info!(
        conversation_id = id,
        branch = %arriving.branch,
        worktree = %worktree.display(),
        untracked = arriving.untracked.len(),
        companions = arriving.companions.len(),
        "the checkout of a Conversation transferred from a member landed here",
    );

    state
        .nudges
        .announce(verkstead_schema::Nudge::Conversation { conversation: id });

    StatusCode::OK.into_response()
}

/// What a checkout weighs once it is off the wire: the bundle and the patch as
/// bytes, and the untracked files with their paths already judged.
struct Carried {
    commit: String,
    bundle: Option<Vec<u8>>,
    patch: Option<Vec<u8>>,
    untracked: Vec<(String, Vec<u8>)>,
}

/// And a Companion's, with the Repo of this registry it named beside it.
struct CarriedAlongside {
    /// The Repo **here** it is a Companion in, which is the id the sending
    /// device's matching settled and this device's own `companions` row is
    /// against.
    repo: i64,

    /// The branch it is on, or `None` for a read-only Companion, which is
    /// detached and holds none.
    branch: Option<String>,

    /// What its checkout was cut from, where the sending device's record knew.
    base_commit: Option<String>,

    carried: Carried,
}

/// One checkout this arrival is to make: which repository, where, and what it is
/// to hold when it is finished.
///
/// **Named before anything is made**, which is what a grill start does for the
/// same reason: a Conversation whose own checkout landed and whose Companion's
/// did not is one no session could be launched in, so every directory is decided
/// and then every directory is cut.
struct Making {
    /// The repository on this machine it is cut from.
    repo: PathBuf,

    /// And where it goes, which this device named off its own Data Directory.
    path: PathBuf,

    /// The branch it is to be on, or `None` for a read-only Companion — which is
    /// checked out detached at [`Carried::commit`] and has nothing to commit.
    branch: Option<String>,

    /// The Companion's Repo id here, and `None` for the Conversation's own
    /// Worktree — which is recorded on the row the store keeps one of per
    /// Conversation rather than beside the Companions.
    companion: Option<i64>,

    /// What it was cut from, where the record that came over knows — carried
    /// through to the row rather than looked up again, a Companion's base being
    /// a name that moves and this the only thing that says what it came to.
    ///
    /// `None` on the Conversation's own, whose base goes on its own row.
    base_commit: Option<String>,

    carried: Carried,
}

/// The base64 read back, and every path in it held against what a path arriving
/// from another machine may be.
///
/// `Err` is the sentence the refusal carries, and it refuses the whole checkout:
/// a member sending something this device will not write is a move that did not
/// happen rather than a move that half did.
fn unpacked(arriving: &CheckoutAcross) -> Result<(Carried, Vec<CarriedAlongside>), String> {
    let carried = off_the_wire(
        &arriving.commit,
        arriving.bundle.as_deref(),
        arriving.patch.as_deref(),
        &arriving.untracked,
    )?;

    let mut alongside = Vec::with_capacity(arriving.companions.len());

    for companion in &arriving.companions {
        alongside.push(CarriedAlongside {
            repo: companion.repo,
            branch: companion.branch.clone(),
            base_commit: companion.base_commit.clone(),
            carried: off_the_wire(
                &companion.commit,
                companion.bundle.as_deref(),
                companion.patch.as_deref(),
                &companion.untracked,
            )?,
        });
    }

    Ok((carried, alongside))
}

/// One checkout's bytes and paths, read back and judged.
///
/// The Conversation's own and each Companion's go through this same reading,
/// because what arrives is a member's message either way: a commit that is no
/// commit id and a path that climbs out of the directory are refused here rather
/// than handed to git.
fn off_the_wire(
    commit: &str,
    bundle: Option<&str>,
    patch: Option<&str>,
    files: &[UntrackedFile],
) -> Result<Carried, String> {
    if !is_an_object_id(commit) {
        return Err(format!(
            "the commit {commit} the checkout is to stand at is no commit id",
        ));
    }

    let decoded = |what: &str, bytes: &str| {
        STANDARD
            .decode(bytes)
            .map_err(|why| format!("the {what} did not read: {why}"))
    };

    let bundle = bundle.map(|bytes| decoded("bundle", bytes)).transpose()?;
    let patch = patch.map(|bytes| decoded("patch", bytes)).transpose()?;

    let mut untracked = Vec::with_capacity(files.len());

    for file in files {
        let UntrackedFile { path, bytes } = file;

        if !inside(path) {
            return Err(format!("{path} is not a path inside a Worktree"));
        }

        untracked.push((path.clone(), decoded(path, bytes)?));
    }

    Ok(Carried {
        commit: commit.to_owned(),
        bundle,
        patch,
        untracked,
    })
}

/// Put every Companion that arrived beside the Conversation's own checkout, with
/// a directory of this device's own naming.
///
/// **Matched against this device's own `companions` rows** rather than trusted
/// off the wire: the **mode** is what decides whether a directory holds a branch
/// or is detached, and the mode is the record's — it arrived with the slice a leg
/// earlier, under a Repo of this registry. A checkout that arrived in the other
/// shape from the one its row calls for is refused by name rather than made in the
/// shape that came, which is how a read-only Companion is kept from being handed a
/// branch nothing here would ever commit on.
///
/// **Every Companion once and every Companion at all.** A row here that nothing
/// arrived for is a move that would land a Conversation with a repository missing
/// from under it, and a repository named twice is one directory recorded and
/// another left behind. Both are refused by name rather than half made: the
/// sending device reads the refusal as *the move did not happen* and the work
/// stays where it was.
///
/// `making` is added to rather than returned, and it is what stops two Companions
/// being handed one directory — see [`crate::worktrees::unclaimed_path`], which
/// is the same claim a grill start makes.
fn beside(
    state: &AppState,
    id: i64,
    companions: &[store::Companion],
    arriving: Vec<CarriedAlongside>,
    making: &mut Vec<Making>,
) -> Result<(), String> {
    for carried in arriving {
        let Some(companion) = companions
            .iter()
            .find(|companion| companion.repo.id == carried.repo)
        else {
            return Err(format!(
                "no companion of this Conversation is the repository {repo} the checkout named",
                repo = carried.repo,
            ));
        };

        if making
            .iter()
            .any(|made| made.companion == Some(companion.repo.id))
        {
            return Err(format!(
                "the companion {name} arrived twice, which is one repository and two checkouts",
                name = companion.repo.name,
            ));
        }

        // The shape the record calls for, held against the shape that came.
        match (companion.mode, &carried.branch) {
            (store::CompanionMode::ReadOnly, Some(branch)) => {
                return Err(format!(
                    "the companion {name} is read-only here and arrived on the branch {branch}, \
                     which is a branch nothing here could ever commit on",
                    name = companion.repo.name,
                ));
            }

            (store::CompanionMode::ReadWrite, None) => {
                return Err(format!(
                    "the companion {name} is read-write here and arrived holding no branch, so \
                     there would be nowhere in it for the work to be committed",
                    name = companion.repo.name,
                ));
            }

            _ => {}
        }

        // Named for what the checkout holds, as a start names it: the branch
        // where there is one, and the base's name where the checkout is
        // detached — a read-only Companion holds no branch to be named for.
        let holding = carried.branch.clone().unwrap_or_else(|| {
            companion
                .base_ref
                .clone()
                .unwrap_or_else(|| companion.repo.default_branch.clone())
        });

        let claimed: Vec<PathBuf> = making.iter().map(|made| made.path.clone()).collect();

        making.push(Making {
            repo: companion.repo.path.clone(),
            path: crate::worktrees::unclaimed_path(
                &state.data_dir,
                id,
                &companion.repo.name,
                &holding,
                &claimed,
            ),
            branch: carried.branch,
            companion: Some(companion.repo.id),
            base_commit: carried.base_commit,
            carried: carried.carried,
        });
    }

    // And nothing left behind, which is the reading the other way round: a
    // Companion on the record with no checkout on the way is a repository the
    // work needs and this device would have nowhere to do it in.
    for companion in companions {
        if !making
            .iter()
            .any(|made| made.companion == Some(companion.repo.id))
        {
            return Err(format!(
                "nothing arrived for the companion {name}, which this Conversation's work is \
                 done alongside",
                name = companion.repo.name,
            ));
        }
    }

    Ok(())
}

/// Where each Companion's checkout went and what it was cut from, for the record
/// that follows the work.
///
/// The commit as well as the directory, which is a start's rule and the same fact
/// by it: a Companion's base is a *name* on its row and a name moves, so the
/// commit that name came to is the only thing that records where a read-only one
/// is standing. What arrived is what is written, that being a fact about the work
/// rather than about either machine.
fn recorded(making: &[Making]) -> Vec<store::CompanionWorktree> {
    making
        .iter()
        .filter_map(|made| {
            Some(store::CompanionWorktree {
                repo_id: made.companion?,
                path: made.path.clone(),
                base_commit: made.base_commit.clone(),
            })
        })
        .collect()
}

/// Whether a path off the wire is one that names a file **inside** a Worktree.
///
/// Git spells a path relative to the checkout with a `/` between the segments on
/// every platform, so that is what is read here rather than the local
/// separator — and every segment has to be a plain name. A `..`, a `.`, an
/// absolute path, a Windows drive prefix and anything under `.git` are all
/// refused: what arrives is a member's message rather than this device's own
/// reading, and a path that climbs is a write outside the directory this device
/// chose.
fn inside(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }

    path.split('/').all(|segment| {
        !segment.is_empty()
            // In any spelling, which is git's own rule: a path with `.git` in it
            // is refused whatever case it arrived in.
            && !segment.eq_ignore_ascii_case(".git")
            && Path::new(segment)
                .components()
                .all(|part| matches!(part, Component::Normal(_)))
    })
}

/// Every checkout of the arrival: the Conversation's own, and one per Companion.
///
/// **Everything or nothing.** A Companion that would not be made takes back the
/// ones already there, the Conversation's own among them, because the sending
/// device reads a refusal as *the move did not happen* and sweeps its copy: a
/// directory left behind under a Conversation that has been swept is a checkout
/// nobody can account for.
///
/// Blocking, which is what every line of it is.
fn landed(making: &[Making]) -> Result<(), String> {
    for (nth, made) in making.iter().enumerate() {
        if let Err(why) = one(made) {
            // This one included, and first: a checkout that fell over may have
            // made its directory and not written into it.
            taken_back(&making[..=nth]);

            return Err(why);
        }
    }

    Ok(())
}

/// Take back every directory of `made`, the branches staying where they are.
///
/// A branch that came out of a bundle is history this repository now has whatever
/// became of the move, and one that was already here was never this device's to
/// take away. What goes is the directory, which is the thing that was made for a
/// move that did not finish.
fn taken_back(made: &[Making]) {
    for done in made.iter().rev() {
        if done.path.exists() {
            crate::worktrees::remove(&done.repo, &done.path);
        }
    }
}

/// One checkout: the branch onto this repository, the directory cut, and the
/// working changes put back into it.
///
/// **In that order and no other.** The ref has to be there before a checkout can
/// be made of it, and the tree has to be there before a patch can be applied to
/// it.
///
/// **And a read-only Companion is the short way through.** It holds no branch, so
/// there is no ref to write and no bundle that came: what it needs is the commit
/// it was detached at, which is refused by name where this device has not got it.
/// Nothing is applied to it either — a patch to a directory bound read-only would
/// be changes a session was never able to make.
fn one(made: &Making) -> Result<(), String> {
    let Making {
        repo,
        path,
        branch,
        carried,
        ..
    } = made;

    let Some(branch) = branch else {
        if crate::worktrees::resolve(repo, &carried.commit).is_none() {
            return Err(format!(
                "this device does not hold the commit {commit} a read-only companion stands at",
                commit = carried.commit,
            ));
        }

        if !crate::worktrees::add_detached(repo, path, &carried.commit) {
            return Err(format!(
                "the commit {commit} could not be checked out on this device",
                commit = carried.commit,
            ));
        }

        return Ok(());
    };

    let named = format!("refs/heads/{branch}");

    fetched(repo, &named, carried)?;

    // What the whole leg is for, asked rather than assumed: the branch on this
    // machine is at the commit it is at on the other one, or nothing here is
    // worth cutting.
    match crate::worktrees::resolve(repo, &named) {
        Some(at) if at == carried.commit => {}

        Some(at) => {
            return Err(format!(
                "{branch} landed at {at} rather than at {commit}",
                commit = carried.commit,
            ));
        }

        None => return Err(format!("{branch} is not in this device's repository")),
    }

    if !crate::worktrees::check_out(repo, path, branch) {
        return Err(format!("{branch} could not be checked out on this device",));
    }

    working(path, carried)
}

/// The branch onto this repository: out of the bundle where one came, and off
/// the commit where the far end had nothing to pack.
///
/// A bundle whose prerequisites this device turns out not to have is refused by
/// git, and the sentence says so: what it means is that the tips this device
/// answered with are not the ones it has, which is a repository that moved
/// under the negotiation.
fn fetched(repo: &Path, named: &str, carried: &Carried) -> Result<(), String> {
    let Some(bundle) = &carried.bundle else {
        // Nothing to unpack, which is a far end standing on the branch's own tip
        // already. The ref is the whole of what is left, and it is refused where
        // this device does not have the commit after all.
        if crate::worktrees::resolve(repo, &carried.commit).is_none() {
            return Err(format!(
                "this device does not hold the commit {commit} the branch stands at, and no \
                 bundle came with it",
                commit = carried.commit,
            ));
        }

        return repos::run(
            repo,
            &["update-ref", "--end-of-options", named, &carried.commit],
        )
        .map_err(|why| format!("the branch could not be written: {why}"));
    };

    // Git takes a bundle as a file rather than on a pipe, so it is one here —
    // in a directory of its own that goes when this function does, whichever way
    // it leaves.
    let held = tempfile::Builder::new()
        .prefix("verkstead-bundle")
        .tempdir()
        .map_err(|why| format!("this device could not make room for the bundle: {why}"))?;

    let file = held.path().join("arriving.bundle");

    std::fs::write(&file, bundle)
        .map_err(|why| format!("the bundle could not be written down here: {why}"))?;

    repos::run(
        repo,
        &[
            "fetch",
            "--no-tags",
            "--no-write-fetch-head",
            "--end-of-options",
            &file.display().to_string(),
            &format!("+{named}:{named}"),
        ],
    )
    .map_err(|why| format!("the branch could not be fetched out of the bundle: {why}"))
}

/// And the working changes back into the tree: the patch first, then every
/// untracked file this device does not ignore.
///
/// **The patch first** because it is what may move the ignore rules: a session
/// that edited `.gitignore` without committing it is a tree whose rules are the
/// ones in the patch rather than the ones on the branch.
///
/// **And this machine's own rules decide.** The files that travelled were picked
/// out by the sending machine's, and a file this checkout would hide is one this
/// device builds for itself.
fn working(worktree: &Path, carried: &Carried) -> Result<(), String> {
    if let Some(patch) = &carried.patch {
        let held = tempfile::Builder::new()
            .prefix("verkstead-patch")
            .tempdir()
            .map_err(|why| format!("this device could not make room for the patch: {why}"))?;

        let file = held.path().join("working.patch");

        std::fs::write(&file, patch)
            .map_err(|why| format!("the patch could not be written down here: {why}"))?;

        // Into the working tree rather than the index: what crossed is *what the
        // files say*, which is what a human means by uncommitted changes and
        // what the Diff a Question Set carries is read off. The line endings are
        // git's own to settle — a binary patch is index form, and this is where
        // whatever convention this tree keeps is put back on.
        repos::run(
            worktree,
            &[
                "apply",
                "--whitespace=nowarn",
                "--end-of-options",
                &file.display().to_string(),
            ],
        )
        .map_err(|why| format!("the working changes could not be applied: {why}"))?;
    }

    if carried.untracked.is_empty() {
        return Ok(());
    }

    let asking: Vec<String> = carried
        .untracked
        .iter()
        .map(|(path, _)| path.clone())
        .collect();

    let hidden = crate::files::ignored(worktree, &asking);

    for (path, bytes) in &carried.untracked {
        if hidden.contains(path) {
            tracing::debug!(
                path,
                "an untracked file that travelled is ignored on this device, so it was not written",
            );

            continue;
        }

        // Segment by segment, git's spelling being `/` on every platform and
        // this machine's being its own — and every segment already held against
        // what one may be, in [`inside`].
        let at = path
            .split('/')
            .fold(worktree.to_owned(), |at: PathBuf, segment| at.join(segment));

        if let Some(parent) = at.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|why| format!("the directory of {path} could not be made: {why}"))?;
        }

        std::fs::write(&at, bytes)
            .map_err(|why| format!("the untracked file {path} could not be written: {why}"))?;
    }

    Ok(())
}

/// Whether a word off the wire is the spelling of a git object id.
///
/// Hex and nothing else, and long enough to be an id rather than a name — the
/// same reading the sending end makes of the tips this one answers with, and for
/// the same reason: what arrives goes on a command line.
fn is_an_object_id(commit: &str) -> bool {
    commit.len() >= 40
        && commit.len() <= 64
        && commit.chars().all(|letter| letter.is_ascii_hexdigit())
}

/// The routes, over the state the workbench answers out of — mounted by
/// [`super::workbench::served`] beside the Repos and the transfers, and for
/// their reason: what they read and write is *this* device's own.
pub(crate) fn route() -> Router<AppState> {
    Router::new().route(TIPS, get(held)).route(
        ONE_TRANSFER_CHECKOUT,
        // Over the router's own default, the way the record's route raises
        // theirs: a checkout is a bundle and a tree's worth of untracked files,
        // and the bound over it is the checkout's own.
        post(cut).layer(DefaultBodyLimit::max(MOST_A_CHECKOUT_IS)),
    )
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// Run git in `dir`, and fail the test where it does not.
    fn run(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git should be on the PATH for these tests");

        assert!(status.success(), "git {args:?} in {}", dir.display());
    }

    /// The file as it was committed and as the session left it — a text one and
    /// a binary one, because the two land by different rules.
    const COMMITTED: &str = "one\ntwo\n";
    const LEFT: &str = "one\ntwo\nthree\n";
    const COMMITTED_BYTES: &[u8] = &[0x00, 0x01, 0x02];
    const LEFT_BYTES: &[u8] = &[0xff, 0xfe, 0x00, 0x7f, 0x80];

    /// **A binary patch crosses a line-ending convention in one piece**, which
    /// is the thing a move between a Windows machine and a Unix one stands on.
    ///
    /// The patch is made where `core.autocrlf` is off and applied where it is
    /// on, which is that crossing without a second machine: a binary patch is
    /// written in index form, so `git apply` puts back whatever convention the
    /// tree it is applied in keeps — the text file lands with this tree's
    /// endings and the binary one lands byte for byte, neither being anything
    /// this code does for itself.
    #[test]
    fn a_binary_patch_crosses_a_line_ending_convention() {
        let dir = tempfile::tempdir().unwrap();
        let sending = dir.path().join("sending");

        std::fs::create_dir_all(&sending).unwrap();
        run(&sending, &["init", "--initial-branch", "main"]);
        run(
            &sending,
            &["config", "user.email", "test@verkstead.invalid"],
        );
        run(&sending, &["config", "user.name", "Verkstead Test"]);
        run(&sending, &["config", "core.autocrlf", "false"]);

        std::fs::write(sending.join("notes.md"), COMMITTED).unwrap();
        std::fs::write(sending.join("fixture.bin"), COMMITTED_BYTES).unwrap();
        run(&sending, &["add", "-A"]);
        run(&sending, &["commit", "-m", "the work so far"]);

        std::fs::write(sending.join("notes.md"), LEFT).unwrap();
        std::fs::write(sending.join("fixture.bin"), LEFT_BYTES).unwrap();

        let patch = repos::bytes(&sending, &["diff", "--binary", "HEAD"], &[0])
            .expect("the tree has something uncommitted in it");

        // And the receiving machine, which keeps the other convention: a clone
        // with the switch on, checked out again so the tree is spelled its way.
        let receiving = dir.path().join("receiving");

        run(
            dir.path(),
            &["clone", &sending.display().to_string(), "receiving"],
        );
        run(&receiving, &["config", "core.autocrlf", "true"]);
        run(&receiving, &["checkout", "HEAD", "--", "."]);

        working(
            &receiving,
            &Carried {
                commit: "0".repeat(40),
                bundle: None,
                patch: Some(patch),
                untracked: Vec::new(),
            },
        )
        .expect("the patch applies");

        assert_eq!(
            std::fs::read_to_string(receiving.join("notes.md")).unwrap(),
            LEFT.replace('\n', "\r\n"),
            "the text file landed in the receiving tree's own convention",
        );
        assert_eq!(
            std::fs::read(receiving.join("fixture.bin")).unwrap(),
            LEFT_BYTES,
            "and the binary one landed byte for byte",
        );
    }

    /// Both are a member's calls, so they go where a member's calls go rather
    /// than through any of the three the gate stands aside for: writing a branch
    /// into somebody's repository is not a stranger's press.
    #[test]
    fn the_checkouts_are_not_one_of_the_un_gated_three() {
        for path in [TIPS, ONE_TRANSFER_CHECKOUT] {
            assert_ne!(path, super::super::IDENTITY);
            assert_ne!(path, super::super::joining::JOIN);
            assert_ne!(path, super::super::exchange::SETTLED);
        }
    }

    /// And both are outside the three prefixes this device keeps to itself.
    #[test]
    fn the_checkouts_are_not_in_a_namespace_kept_back() {
        for prefix in super::super::workbench::KEPT_TO_ITSELF {
            for path in [TIPS, ONE_TRANSFER_CHECKOUT] {
                assert!(
                    !path.starts_with(prefix),
                    "{path} would be held back with {prefix}/",
                );
            }
        }
    }

    /// The checkout lands under the transfer it belongs to, which is what says
    /// the two are one move rather than two presses.
    #[test]
    fn the_checkout_is_under_the_transfer() {
        assert_eq!(
            ONE_TRANSFER_CHECKOUT,
            format!("{}/checkout", super::super::transfers::ONE_TRANSFER),
        );
    }

    /// And the tips are asked for by the Repo they are of, filled in the way the
    /// asking end fills it.
    #[test]
    fn the_tips_name_the_repo() {
        assert!(TIPS.contains("{repo}"));
        assert_eq!(tips_of(7), "/api/peer/v1/repos/7/tips");
    }

    /// **A path that climbs is no path inside a Worktree**, nor is an absolute
    /// one, nor anything under `.git`: what arrives is a member's message, and a
    /// write outside the directory this device chose is the failure that rule is
    /// there for.
    #[test]
    fn only_a_path_inside_the_worktree_is_written() {
        for path in ["notes.md", "crates/server/src/lib.rs", "a b/c.txt"] {
            assert!(inside(path), "{path} is inside a Worktree");
        }

        for path in [
            "",
            "..",
            "../elsewhere",
            "crates/../../elsewhere",
            "./notes.md",
            "/etc/passwd",
            "crates//server",
            ".git/config",
            ".GIT/config",
            "crates/.git/hooks/pre-commit",
        ] {
            assert!(!inside(path), "{path} is no path inside a Worktree");
        }

        // And a Windows-spelled path, which is this machine's own question: a
        // drive and a separator are components there and are one plain name
        // here, so it is refused where it could name anything and is a strange
        // file name where it could not.
        #[cfg(windows)]
        for path in [r"C:\Windows\system32", r"crates\..\..\elsewhere"] {
            assert!(!inside(path), "{path} is no path inside a Worktree");
        }
    }

    /// And a commit id is hex of a hash's length and nothing else — the same
    /// reading the sending end makes of a tip.
    #[test]
    fn only_an_object_id_is_taken_for_one() {
        assert!(is_an_object_id(&"a".repeat(40)));
        assert!(is_an_object_id(&"F".repeat(64)));

        assert!(!is_an_object_id("HEAD"));
        assert!(!is_an_object_id("--upload-pack=rm -rf /"));
        assert!(!is_an_object_id(&"a".repeat(39)));
    }
}
