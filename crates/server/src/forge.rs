//! The Forge as an Activity reaches it: the `forge-*` host functions of the
//! `activity` world, wrapping [`crate::github`] (ADR-0030, *Forge*).
//!
//! **A wrapper and no more, until stage 12.** Every call here asks what the
//! in-process watchers ask — [`crate::checks`], [`crate::comments`],
//! [`crate::merges`] — in the same words, as the Repo's Author and off the
//! runtime's threads, and answers it in the world's shapes. Stage 12 puts the
//! Forge interface behind these calls, and the component calling them does not
//! change.
//!
//! **A capability the Forge lacks is settled at once.** A Forge declares what
//! it can do — checks, review, create — and a wrap-up watcher over one it has
//! not got has nothing to wait for: the call says `lacking`, and the watcher
//! it was asked for is settled before the answer goes back, so the wrap-up is
//! never held up by something that will never come. GitHub has every
//! capability, so with only GitHub behind it this path is fed by
//! [`Forge::lacking`], the suite's seam; stage 12 feeds it from the Forge's
//! declaration.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use verkstead_store as store;

use crate::AppState;
use crate::github::{self, Checked, Mergeable, Stands};
use crate::plugins::component::worlds::category::activity::verkstead::plugin::activity_types as world;

/// Something a Forge may or may not be able to do, in ADR-0030's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Running checks against a pull request, which the `checks` watcher
    /// waits on.
    Checks,

    /// Comments and reviews on a pull request, which the `comments` watcher
    /// reads and replies on.
    Review,

    /// Opening a pull request.
    Create,
}

/// What the Forge is said to lack — nothing, behind GitHub.
#[derive(Clone, Default)]
pub(crate) struct Forge {
    lacking: Arc<Mutex<HashSet<Capability>>>,
}

impl Forge {
    pub(crate) fn new() -> Forge {
        Forge::default()
    }

    /// Say the Forge lacks `capability` from now on.
    pub(crate) fn lacking(&self, capability: Capability) {
        self.lacking
            .lock()
            .expect("nothing panics holding it")
            .insert(capability);
    }

    fn lacks(&self, capability: Capability) -> bool {
        self.lacking
            .lock()
            .expect("nothing panics holding it")
            .contains(&capability)
    }
}

/// A pull request an Activity named, found on the Conversation: the Repo it is
/// in — the Conversation's own, or a Companion's — and its number there.
pub(crate) struct Named {
    pub(crate) repo: store::Repo,
    pub(crate) number: i64,
}

/// The Repo `name` is to `conversation_id` — its own, or a Companion's — or
/// the sentence a call naming it is refused with.
pub(crate) async fn repo_named(
    state: &AppState,
    conversation_id: i64,
    name: &str,
) -> Result<store::Repo, String> {
    let conversation = match store::load_conversation(&state.pool, conversation_id).await {
        Ok(Some(conversation)) => conversation,
        Ok(None) => return Err("There is no Conversation left to ask about.".to_owned()),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the Conversation a Repo is named on failed");
            return Err("The Conversation couldn't be read.".to_owned());
        }
    };

    if conversation.repo.name == name {
        return Ok(conversation.repo);
    }

    conversation
        .companions
        .into_iter()
        .find(|companion| companion.repo.name == name)
        .map(|companion| companion.repo)
        .ok_or_else(|| {
            format!("`{name}` is neither this Conversation's repo nor one of its companions.")
        })
}

/// `pull_request` as the Conversation holds it.
pub(crate) async fn named(
    state: &AppState,
    conversation_id: i64,
    pull_request: &world::PullRequest,
) -> Result<Named, String> {
    Ok(Named {
        repo: repo_named(state, conversation_id, &pull_request.repo).await?,
        number: pull_request.number,
    })
}

/// Every pull request the Conversation's work is on, as recorded.
pub(crate) async fn recorded(state: &AppState, conversation_id: i64) -> Vec<world::PullRequest> {
    match store::pull_requests(&state.pool, conversation_id).await {
        Ok(opened) => opened
            .into_iter()
            .map(|(repo, opened)| world::PullRequest {
                repo: repo.name,
                number: opened.number,
            })
            .collect(),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading which pull requests a Conversation is on failed");
            Vec::new()
        }
    }
}

/// The pull request's state and whether it merges. Every Forge can say so.
pub(crate) async fn standing(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Result<world::PullRequestStanding, world::Unforged> {
    let named = named(state, conversation_id, &pull_request)
        .await
        .map_err(world::Unforged::Unasked)?;

    let landing = asked(state, &named, github::landing).await?;

    Ok(world::PullRequestStanding {
        mergeable: mergeable(landing.mergeable),
        stands: match landing.stands {
            Stands::Open => world::PullRequestStands::Open,
            Stands::Merged => world::PullRequestStands::Merged,
            Stands::Closed => world::PullRequestStands::Closed,
            Stands::Unknown => world::PullRequestStands::Unknown,
        },
    })
}

/// Its checks, or the `checks` watcher over it settled at once — and what
/// they came to, and whether it merges, written down for the card.
pub(crate) async fn checks(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Result<world::Suite, world::Unforged> {
    let named = named(state, conversation_id, &pull_request)
        .await
        .map_err(world::Unforged::Unasked)?;

    if state.forge.lacks(Capability::Checks) {
        return lacking(state, conversation_id, &named, |repo_id, number| {
            store::WaitingOn::Checks { repo_id, number }
        })
        .await;
    }

    let suite = asked(state, &named, github::checks).await?;

    // Written down before the component decides anything about it, because the
    // card draws both and outlives the wrap-up: this is the one place anything
    // asks how the checks are while one runs.
    crate::checks::remember(
        state,
        conversation_id,
        named.repo.id,
        named.number,
        &suite.checks,
    )
    .await;
    crate::checks::marked(
        state,
        conversation_id,
        named.repo.id,
        named.number,
        suite.mergeable,
    )
    .await;

    Ok(world::Suite {
        head: suite.head,
        checks: suite
            .checks
            .into_iter()
            .map(|check| world::Check {
                name: check.name,
                how: match check.how {
                    Checked::Passed => world::Checked::Passed,
                    Checked::Failed => world::Checked::Failed,
                    Checked::Running => world::Checked::Running,
                },
                link: check.link,
            })
            .collect(),
        mergeable: mergeable(suite.mergeable),
    })
}

/// Everything said on it, or the `comments` watcher over it settled at once.
pub(crate) async fn comments(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
) -> Result<Vec<world::Comment>, world::Unforged> {
    let named = named(state, conversation_id, &pull_request)
        .await
        .map_err(world::Unforged::Unasked)?;

    if state.forge.lacks(Capability::Review) {
        return lacking(state, conversation_id, &named, comments_on).await;
    }

    let said = asked(state, &named, github::comments).await?;

    Ok(said
        .into_iter()
        .map(|comment| world::Comment {
            which: comment.which,
            author: comment.author,
            at: comment.at,
            about: comment.about,
            markdown: comment.markdown,
        })
        .collect())
}

/// Say `markdown` on it, as its Repo's Author.
pub(crate) async fn reply(
    state: &AppState,
    conversation_id: i64,
    pull_request: world::PullRequest,
    markdown: String,
) -> Result<String, world::Unforged> {
    let named = named(state, conversation_id, &pull_request)
        .await
        .map_err(world::Unforged::Unasked)?;

    if state.forge.lacks(Capability::Review) {
        return lacking(state, conversation_id, &named, comments_on).await;
    }

    asked(state, &named, move |gh, repo, number| {
        github::reply(gh, repo, number, &markdown)
    })
    .await
}

/// The pull request open on `branch` of `repo`, where there is one.
pub(crate) async fn opened(
    state: &AppState,
    conversation_id: i64,
    repo: String,
    branch: String,
) -> Result<Option<world::Opened>, world::Unforged> {
    let repo = repo_named(state, conversation_id, &repo)
        .await
        .map_err(world::Unforged::Unasked)?;

    // Nothing to settle: no watcher waits on a pull request being opened.
    if state.forge.lacks(Capability::Create) {
        return Err(world::Unforged::Lacking);
    }

    let gh = state.github.as_author(repo.author.as_deref());
    let path = repo.path.clone();

    let found = tokio::task::spawn_blocking(move || github::pull_request(&gh, &path, &branch))
        .await
        .map_err(|_| world::Unforged::Unasked("The Forge couldn't be asked.".to_owned()))?;

    // GitHub answers a branch with nothing open on it the way it answers any
    // question it could not: `gh pr view` fails. Either is no pull request to
    // the caller, which asks again.
    Ok(found.ok().map(|opened| world::Opened {
        number: opened.number,
        title: opened.title,
        url: opened.url,
        head: opened.head,
        base: opened.base,
    }))
}

/// Ask `asking` about `named` as its Repo's Author, off the runtime's threads,
/// and say why not in the world's words where `gh` would not answer.
async fn asked<T, F>(state: &AppState, named: &Named, asking: F) -> Result<T, world::Unforged>
where
    T: Send + 'static,
    F: FnOnce(&github::Gh, &std::path::Path, i64) -> Result<T, github::Trouble> + Send + 'static,
{
    let gh = state.github.as_author(named.repo.author.as_deref());
    let path: PathBuf = named.repo.path.clone();
    let number = named.number;

    match tokio::task::spawn_blocking(move || asking(&gh, &path, number)).await {
        Ok(Ok(answered)) => Ok(answered),
        Ok(Err(trouble)) => Err(world::Unforged::Unasked(trouble.why())),
        Err(_) => Err(world::Unforged::Unasked(
            "The Forge couldn't be asked.".to_owned(),
        )),
    }
}

/// Settle the watcher `waiting_on` names over `named`, there being nothing for
/// it to wait for, and answer `lacking`.
async fn lacking<T>(
    state: &AppState,
    conversation_id: i64,
    named: &Named,
    waiting_on: fn(i64, i64) -> store::WaitingOn,
) -> Result<T, world::Unforged> {
    let waiting_on = waiting_on(named.repo.id, named.number);

    if let Err(error) = store::settle_wrap_up(&state.pool, conversation_id, waiting_on).await {
        tracing::error!(error = ?error, conversation_id, ?waiting_on, "settling a watcher the Forge can't feed failed");
    }

    Err(world::Unforged::Lacking)
}

fn comments_on(repo_id: i64, number: i64) -> store::WaitingOn {
    store::WaitingOn::Comments { repo_id, number }
}

fn mergeable(mergeable: Mergeable) -> world::Mergeable {
    match mergeable {
        Mergeable::Cleanly => world::Mergeable::Cleanly,
        Mergeable::Conflicting => world::Mergeable::Conflicting,
        Mergeable::Unknown => world::Mergeable::Unknown,
    }
}
