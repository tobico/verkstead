//! Starting a Conversation and changing what the human may change about one:
//! everything between a workbench form and a row in the store.
//!
//! Two of the three edits are decided against the repository rather than taken
//! on trust, and git is the one asked both times — whether a name is one it
//! would take for a branch, and whether anything in the repository answers to
//! what was typed as a base commit. Refused here rather than at grill start,
//! where a bad name or a commit that is not there would be a failure with nobody
//! watching.
//!
//! Starting the grilling is where a Conversation stops being a record and gets
//! somewhere to work — see [`start_grilling`] — and aborting is where it gives
//! that back. Neither runs a session: the branch and the worktree are the ground
//! a session will later stand on, and the stage that stands on it is the next
//! one.

use std::path::{Path, PathBuf};

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{
    BaseRecorded, BranchRenamed, BriefSaved, ConversationAborted, GrillingStarted, ProfileEntry,
    Started, Worktree,
};

use crate::repos::git;
use crate::store;
use crate::watched::WatchedPaths;
use crate::worktrees;

/// Start a Conversation against a registered Repo, on a branch name nobody has
/// had to think of yet.
///
/// The name is the server's because the record is: a prefill the browser
/// invented would be a name the server never saw, and the human may well leave
/// it as it is.
pub(crate) async fn start(pool: &SqlitePool, repo_id: i64) -> Result<Started> {
    Ok(
        match store::start_conversation(pool, repo_id, &branch_name()).await? {
            Some(id) => Started::Started { id },
            None => Started::NoSuchRepo,
        },
    )
}

/// Save what the human has written into the Brief.
pub(crate) async fn save_brief(pool: &SqlitePool, id: i64, markdown: &str) -> Result<BriefSaved> {
    Ok(match store::save_brief(pool, id, markdown).await? {
        store::Edited::Saved => BriefSaved::Saved,
        store::Edited::NoSuchConversation => BriefSaved::NoSuchConversation,
        store::Edited::NotDrafting => BriefSaved::NotDrafting,
    })
}

/// Name the branch the work will be done on.
///
/// Whether the name is usable is git's to say, not a list of forbidden
/// characters here: the branch this names is one git will be asked to create,
/// and the only opinion that will matter then is the one being asked now.
pub(crate) async fn rename_branch(
    pool: &SqlitePool,
    id: i64,
    branch: &str,
) -> Result<BranchRenamed> {
    let branch = branch.trim().to_owned();

    if !tokio::task::spawn_blocking({
        let branch = branch.clone();
        move || is_branch_name(&branch)
    })
    .await?
    {
        return Ok(BranchRenamed::NotABranchName);
    }

    Ok(match store::rename_branch(pool, id, &branch).await? {
        store::Edited::Saved => BranchRenamed::Renamed,
        store::Edited::NoSuchConversation => BranchRenamed::NoSuchConversation,
        store::Edited::NotDrafting => BranchRenamed::NotDrafting,
    })
}

/// Record the commit the work branches from, or put the Conversation back on the
/// default-branch rule.
///
/// What is stored is the commit the repository resolved, not what was typed: a
/// tag or a branch name is a moving target, and the point of overriding the rule
/// is to pin the work to one commit. Blank counts as clearing it — a field
/// emptied is the human taking the override away, not naming a commit called
/// nothing.
pub(crate) async fn set_base_commit(
    pool: &SqlitePool,
    id: i64,
    asked: Option<&str>,
) -> Result<BaseRecorded> {
    let asked = asked.map(str::trim).filter(|asked| !asked.is_empty());

    let commit = match asked {
        None => None,
        Some(asked) => {
            // The repository to ask is the Conversation's own, so the
            // Conversation has to be there before there is anywhere to ask.
            let Some(conversation) = store::load_conversation(pool, id).await? else {
                return Ok(BaseRecorded::NoSuchConversation);
            };

            let asked = asked.to_owned();
            let resolved =
                tokio::task::spawn_blocking(move || resolve(&conversation.repo.path, &asked))
                    .await?;

            match resolved {
                Some(commit) => Some(commit),
                None => return Ok(BaseRecorded::NoSuchCommit),
            }
        }
    };

    Ok(
        match store::set_base_commit(pool, id, commit.as_deref()).await? {
            store::Edited::Saved => BaseRecorded::Recorded,
            store::Edited::NoSuchConversation => BaseRecorded::NoSuchConversation,
            store::Edited::NotDrafting => BaseRecorded::NotDrafting,
        },
    )
}

/// Give a drafting Conversation somewhere to work: a branch off its base commit
/// and a worktree of its Repo, and the move onto the Timeline that says so.
///
/// Everything that has to be true is checked here, each refused by its own name,
/// because each is something different for the human to go and do. They are
/// checked in the order they can be: the record first, then the Profiles, then
/// the Brief, then the repository — so the answers that cost a git call are
/// asked only once the cheap ones have passed.
///
/// The order of the doing is the other way round from the recording. Git makes
/// the branch and the worktree, and only then does the store hear about it: a
/// row saying a Conversation is grilling when nothing was ever checked out would
/// be a Conversation nothing could start and nothing would clean up. The reverse
/// — a worktree on disk that the store does not know about — is a directory to
/// tidy, which is the lesser of the two.
pub(crate) async fn start_grilling(
    pool: &SqlitePool,
    watched: &WatchedPaths,
    state: &Path,
    id: i64,
) -> Result<GrillingStarted> {
    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(GrillingStarted::NoSuchConversation);
    };

    if conversation.state != store::Lifecycle::Draft {
        return Ok(GrillingStarted::NotDrafting);
    }

    // Read as rows rather than judged off the ids, which is the same reading the
    // pane gets — a Profile whose pair has gone is not one to launch a session
    // under, and the id alone cannot say so.
    let grilling = crate::profiles::entry(watched, conversation.grilling_profile.clone()).await?;
    let implementation =
        crate::profiles::entry(watched, conversation.implementation_profile.clone()).await?;

    if let Some(refusal) = unready(grilling.as_ref(), implementation.as_ref()) {
        return Ok(refusal);
    }

    if brief(pool, id).await?.trim().is_empty() {
        return Ok(GrillingStarted::EmptyBrief);
    }

    // What the work branches from. An override is re-resolved rather than
    // trusted: it resolved when the human typed it, and a commit can be gone by
    // now. Without one it is the default branch's tip, which is a rule that has
    // never resolved to anything until this moment.
    let named = conversation
        .base_commit
        .clone()
        .unwrap_or_else(|| conversation.repo.default_branch.clone());

    let repo = conversation.repo.path.clone();
    let branch = conversation.branch.clone();
    let path = worktrees::worktree_path(state, id, &conversation.repo.name, &branch);

    // The filesystem and git halves together, off the runtime: a worktree of a
    // large repository is not a quick call, and every part of this blocks.
    let made = tokio::task::spawn_blocking({
        let path = path.clone();
        move || {
            let Some(commit) = worktrees::resolve(&repo, &named) else {
                return Err(GrillingStarted::NoBaseCommit);
            };

            if worktrees::branch_exists(&repo, &branch) {
                return Err(GrillingStarted::BranchExists);
            }

            match worktrees::add(&repo, &path, &branch, &commit) {
                true => Ok(commit),
                false => Err(GrillingStarted::WorktreeRefused),
            }
        }
    })
    .await?;

    let commit = match made {
        Ok(commit) => commit,
        Err(refusal) => return Ok(refusal),
    };

    Ok(
        match store::start_grilling(pool, id, &commit, &path).await? {
            store::Grilling::Started => GrillingStarted::Started,
            store::Grilling::NoSuchConversation => GrillingStarted::NoSuchConversation,
            store::Grilling::NotDrafting => GrillingStarted::NotDrafting,
        },
    )
}

/// Stop a Conversation wherever it has got to: its worktree removed, its branch
/// left where it is.
///
/// The worktree goes before the record does, for the reason the branch is made
/// before one: what is recorded is what happened. A Conversation that said it
/// had stopped while its directory was still on disk would be one nothing would
/// ever come back and remove.
pub(crate) async fn abort(pool: &SqlitePool, id: i64) -> Result<ConversationAborted> {
    let Some(conversation) = store::load_conversation(pool, id).await? else {
        return Ok(ConversationAborted::NoSuchConversation);
    };

    if let Some(path) = conversation.worktree.clone() {
        let repo = conversation.repo.path.clone();

        let removed = tokio::task::spawn_blocking(move || worktrees::remove(&repo, &path)).await?;

        if !removed {
            tracing::error!(
                conversation_id = id,
                "a Conversation's worktree could not be removed"
            );
            return Ok(ConversationAborted::WorktreeStuck);
        }
    }

    Ok(match store::abort_conversation(pool, id).await? {
        store::Aborting::Aborted => ConversationAborted::Aborted,
        store::Aborting::AlreadyAborted => ConversationAborted::AlreadyAborted,
        store::Aborting::NoSuchConversation => ConversationAborted::NoSuchConversation,
    })
}

/// Why these two Profiles are not a pair of accounts to run under, or `None`
/// where they are.
///
/// The same rule [`crate::profiles::ready_to_grill`] answers for the pane, said
/// the other way round: that one says whether to offer the button, this one says
/// what was wrong when it was pressed. Each is named separately, because
/// choosing a Profile and mending a broken one are different jobs.
fn unready(
    grilling: Option<&ProfileEntry>,
    implementation: Option<&ProfileEntry>,
) -> Option<GrillingStarted> {
    let Some(grilling) = grilling else {
        return Some(GrillingStarted::NoGrillingProfile);
    };

    let Some(implementation) = implementation else {
        return Some(GrillingStarted::NoImplementationProfile);
    };

    [grilling, implementation]
        .into_iter()
        .any(|profile| profile.broken.is_some())
        .then_some(GrillingStarted::ProfileBroken)
}

/// The Brief off a Conversation's Timeline.
///
/// Found rather than taken from the front: the Brief is the first Event, but by
/// the time anything asks this there are moves on the Timeline after it.
async fn brief(pool: &SqlitePool, id: i64) -> Result<String> {
    Ok(store::timeline(pool, id)
        .await?
        .into_iter()
        .find_map(|event| match event.event {
            store::Event::Brief(markdown) => Some(markdown),
            _ => None,
        })
        .unwrap_or_default())
}

/// A Conversation's worktree as the viewer receives it: where it is, and whether
/// it is still there.
///
/// The look at the filesystem happens here rather than in the store, because
/// whether a directory exists is not something a database knows — and it is
/// worth knowing: a worktree deleted by hand should read as a Conversation with
/// a problem rather than as an obscure failure from whatever next works in it.
pub(crate) async fn worktree(path: Option<PathBuf>) -> Result<Option<Worktree>> {
    let Some(path) = path else {
        return Ok(None);
    };

    let missing = tokio::task::spawn_blocking({
        let path = path.clone();
        move || !path.is_dir()
    })
    .await?;

    Ok(Some(Worktree {
        // Stored as UTF-8 in the first place — a path that is not cannot be
        // recorded — so nothing is lost putting it back on the wire.
        path: path.to_string_lossy().into_owned(),
        missing,
    }))
}

/// Whether everything needed before grilling starts is settled, as the pane
/// reads it: the two Profiles, and a Brief with something in it.
///
/// Answered against what the endpoint has already read rather than by loading
/// the Conversation again — and it deliberately says nothing about the branch or
/// the base commit, which are decided against git when the button is pressed.
pub(crate) fn ready_to_grill(
    state: store::Lifecycle,
    grilling: Option<&ProfileEntry>,
    implementation: Option<&ProfileEntry>,
    brief: &str,
) -> bool {
    state == store::Lifecycle::Draft
        && !brief.trim().is_empty()
        && crate::profiles::ready_to_grill(grilling, implementation)
}

/// Whether git would take this as a branch name.
///
/// Asked as the full ref the branch would become rather than through
/// `--branch`: that form is judged by the rules alone, with no repository to
/// judge it in and nothing resolved on the way — `--branch` would read `@{-1}`
/// as whichever branch was checked out where the server happens to be running,
/// and hand back a name nobody typed.
///
/// The leading dash is the one rule added here, because it is not one of git's:
/// `refs/heads/-x` is a perfectly well-formed ref, and a branch called `-x` is
/// one every git command line would read as an option.
///
/// The directory is the server's own and means nothing — this asks about the
/// spelling of a name, and there is no repository involved in the answer.
fn is_branch_name(branch: &str) -> bool {
    !branch.is_empty()
        && !branch.starts_with('-')
        && git(
            Path::new("."),
            &["check-ref-format", &format!("refs/heads/{branch}")],
        )
        .is_some()
}

/// The commit `asked` names in the repository at `path`, in full, or `None` if
/// nothing there answers to it.
///
/// `^{commit}` is what makes a tag or a branch resolve to the commit it points
/// at rather than to itself, and what refuses a tree or a blob that happens to
/// share a prefix.
fn resolve(path: &Path, asked: &str) -> Option<String> {
    let commit = git(
        path,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            // Whatever was typed is the human's, so it must not be able to
            // arrive as an option.
            "--end-of-options",
            &format!("{asked}^{{commit}}"),
        ],
    )?;

    let commit = commit.trim();

    (!commit.is_empty()).then(|| commit.to_owned())
}

/// A branch name to start a Conversation under, until the human names it
/// themselves.
///
/// Two words rather than a hash: it is a branch name, so it is going to be typed
/// and read aloud, and the whole point of prefilling one is that the human need
/// not stop and think of anything before they start writing the brief.
fn branch_name() -> String {
    /// Colours, weather and temper — words that pair with anything below and
    /// carry no meaning about the work, so a name that is left as it is never
    /// says something untrue about the branch.
    const QUALITIES: [&str; 32] = [
        "amber", "ashen", "autumn", "brisk", "cobalt", "copper", "crisp", "dusky", "eager",
        "ember", "flint", "frosted", "gilded", "hushed", "indigo", "ivory", "lucid", "mellow",
        "misty", "nimble", "ochre", "opal", "quiet", "rustic", "sable", "scarlet", "slate",
        "sunlit", "teal", "umber", "verdant", "wistful",
    ];

    /// Birds, weather and landscape, for the same reason.
    const THINGS: [&str; 32] = [
        "anchor", "beacon", "bramble", "cedar", "cinder", "coppice", "curlew", "delta", "eddy",
        "fathom", "gable", "harbour", "heron", "kestrel", "lantern", "meadow", "orchard", "otter",
        "pennant", "quarry", "ridge", "rookery", "sable", "shale", "sparrow", "thicket", "thistle",
        "tundra", "vale", "willow", "wren", "zephyr",
    ];

    // A generator that could not answer is not a reason to refuse to start a
    // Conversation: the name is a prefill the human is free to replace, so an
    // unlucky machine gets the first pair rather than an error.
    let picked = getrandom::u64().unwrap_or(0);

    let quality = QUALITIES[(picked % QUALITIES.len() as u64) as usize];
    let thing = THINGS[((picked / QUALITIES.len() as u64) % THINGS.len() as u64) as usize];

    format!("{quality}-{thing}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A prefilled name has to be one git would take, or the human is handed a
    /// branch that cannot be created and no reason why.
    #[test]
    fn every_prefilled_branch_name_is_one_git_would_take() {
        for _ in 0..64 {
            let name = branch_name();
            assert!(is_branch_name(&name), "git refused {name:?}");
        }
    }

    #[test]
    fn a_prefilled_name_is_two_words_a_human_can_say() {
        let name = branch_name();
        let (quality, thing) = name.split_once('-').expect("two words and a hyphen");

        assert!(!quality.is_empty() && !thing.is_empty());
        assert!(name.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
    }

    #[test]
    fn the_names_git_refuses_are_refused_here() {
        for refused in [
            "",
            " ",
            "has space",
            "two..dots",
            "ends/",
            "-dash",
            "with~tilde",
            "@{-1}",
        ] {
            assert!(!is_branch_name(refused), "{refused:?} should be refused");
        }
    }

    #[test]
    fn an_ordinary_branch_name_is_taken() {
        for taken in ["main", "rate-limiting", "feature/rate-limiting", "v2"] {
            assert!(is_branch_name(taken), "{taken:?} should be taken");
        }
    }
}
