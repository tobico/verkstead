//! The Forge as an Activity reaches it: the `forge-*` host functions of the
//! `activity` world, over [`crate::forges::Forge`] (ADR-0030, *Forge*).
//!
//! **A wrapper and no more.** Every call here asks what the in-process
//! watchers ask — [`crate::checks`], [`crate::comments`], [`crate::merges`] —
//! through the same interface, as the Repo's Author and off the runtime's
//! threads, and answers it in the world's shapes. The component calling them
//! does not know which Forge answered.
//!
//! **A capability the Forge lacks is settled at once.** A Forge declares what
//! it can do — `checks`, `review`, and the rest of ADR-0030's five — and a
//! wrap-up watcher over one the Repo's Forge has not got has nothing to wait
//! for: the call says `lacking`, the watcher it was asked for is settled
//! before the answer goes back, and the Timeline says why, so the wrap-up is
//! never held up by something that will never come. Read off the declaration
//! of the Forge the Repo's remote is on, at the moment of the call — see
//! [`crate::forges::of`]. GitHub declares all five.
//!
//! **And a Forge can be registered in-process**, beside the plugins, which is
//! how the suite stands one up declaring what it likes — see [`Registered`].

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use verkstead_store as store;

use crate::AppState;
pub use crate::plugins::manifest::forge::Capability;

use crate::forges::{Checked, Mergeable, Stands};
use crate::plugins::component::worlds::category::activity::verkstead::plugin::activity_types as world;

/// A Forge as the suite declares one: a plugin's `forge` block, without the
/// plugin. See [`Registered`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declared {
    pub uuid: String,
    pub name: String,
    pub noun: String,
    /// The hosts it answers for. A Repo whose remote names no host — a path —
    /// is on the first Forge enabled, which a registered one is.
    pub hosts: Vec<String>,
    pub capabilities: Vec<Capability>,
}

/// The Forges registered on this server in-process rather than through a
/// plugin — none on a served one.
///
/// **What answers for one is the router's own Forge**, the one every operation
/// goes through: a registered Forge is a declaration — its name, its noun, its
/// hosts and what it can do — over the same implementation, which is what the
/// suite needs to stand up a Forge declaring no capabilities over its own
/// GitHub stand-in. Each comes before every built-in in
/// [`crate::forges::enabled`], so a Repo whose remote is a path is on it.
#[derive(Clone, Default)]
pub(crate) struct Registered {
    forges: Arc<Mutex<Vec<crate::forges::Enabled>>>,
}

impl Registered {
    pub(crate) fn new() -> Registered {
        Registered::default()
    }

    /// Register `declared` from now on.
    pub(crate) fn register(&self, declared: Declared) {
        let mut hosts: Vec<String> = Vec::new();
        for host in declared.hosts {
            let host = host.trim().to_ascii_lowercase();
            if !host.is_empty() && !hosts.contains(&host) {
                hosts.push(host);
            }
        }

        self.forges
            .lock()
            .expect("nothing panics holding it")
            .push(crate::forges::Enabled {
                uuid: declared.uuid,
                name: declared.name,
                hosts,
                noun: declared.noun,
                capabilities: declared.capabilities,
            });
    }

    /// Every one registered, in the order they were.
    pub(crate) fn registered(&self) -> Vec<crate::forges::Enabled> {
        self.forges
            .lock()
            .expect("nothing panics holding it")
            .clone()
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

    let landing = asked(state, &named, |forge, repo, number| {
        forge.landing(repo, number)
    })
    .await?;

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

    if let Some(forge) = lacks(state, &named, Capability::Checks).await {
        return lacking(state, conversation_id, &named, &forge, Capability::Checks).await;
    }

    let suite = asked(state, &named, |forge, repo, number| {
        forge.checks(repo, number)
    })
    .await?;

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

    if let Some(forge) = lacks(state, &named, Capability::Review).await {
        return lacking(state, conversation_id, &named, &forge, Capability::Review).await;
    }

    let said = asked(state, &named, |forge, repo, number| {
        forge.comments(repo, number)
    })
    .await?;

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

    if let Some(forge) = lacks(state, &named, Capability::Review).await {
        return lacking(state, conversation_id, &named, &forge, Capability::Review).await;
    }

    asked(state, &named, move |gh, repo, number| {
        gh.reply(repo, number, &markdown)
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

    let gh = state.github.as_author(repo.author.as_deref());
    let path = repo.path.clone();

    let found = tokio::task::spawn_blocking(move || gh.pull_request(&path, &branch))
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
/// and say why not in the world's words where the Forge would not answer.
async fn asked<T, F>(state: &AppState, named: &Named, asking: F) -> Result<T, world::Unforged>
where
    T: Send + 'static,
    F: FnOnce(
            &dyn crate::forges::Forge,
            &std::path::Path,
            i64,
        ) -> Result<T, crate::forges::Trouble>
        + Send
        + 'static,
{
    let gh = state.github.as_author(named.repo.author.as_deref());
    let path: PathBuf = named.repo.path.clone();
    let number = named.number;

    match tokio::task::spawn_blocking(move || asking(&*gh, &path, number)).await {
        Ok(Ok(answered)) => Ok(answered),
        Ok(Err(trouble)) => Err(world::Unforged::Unasked(trouble.why())),
        Err(_) => Err(world::Unforged::Unasked(
            "The Forge couldn't be asked.".to_owned(),
        )),
    }
}

/// The Forge `named`'s Repo is on, where it lacks `capability` — `None` where
/// it has it. A Repo on no Forge is not answered for here: nothing would be
/// asked of one, and what meets it is [`crate::wrapping::unforged`].
async fn lacks(
    state: &AppState,
    named: &Named,
    capability: Capability,
) -> Option<crate::forges::Enabled> {
    match crate::forges::of(state, &named.repo.path).await {
        crate::forges::Matched::On(forge) if !forge.has(capability) => Some(forge),
        crate::forges::Matched::On(_) | crate::forges::Matched::Unmatched { .. } => None,
    }
}

/// Settle the watcher over `named` that waits on what `forge` lacks, there
/// being nothing for it to wait for, and answer `lacking` — saying so on the
/// Timeline the first time, so a wrap-up that went Done without a check or a
/// review says why.
async fn lacking<T>(
    state: &AppState,
    conversation_id: i64,
    named: &Named,
    forge: &crate::forges::Enabled,
    capability: Capability,
) -> Result<T, world::Unforged> {
    let (repo_id, number) = (named.repo.id, named.number);
    let waiting_on = match capability {
        Capability::Checks => store::WaitingOn::Checks { repo_id, number },
        _ => store::WaitingOn::Comments { repo_id, number },
    };

    let already = match store::wrap_up_settled(&state.pool, conversation_id).await {
        Ok(settled) => settled.contains(&waiting_on),
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading what a wrap-up has settled failed");
            false
        }
    };

    if already {
        return Err(world::Unforged::Lacking);
    }

    if let Err(error) = store::settle_wrap_up(&state.pool, conversation_id, waiting_on).await {
        tracing::error!(error = ?error, conversation_id, ?waiting_on, "settling a watcher the Forge can't feed failed");
    } else if let Some(said) =
        crate::forges::unwatched(&forge.name, &forge.noun, capability, number)
        && let Err(error) = store::note(&state.pool, conversation_id, &said).await
    {
        tracing::error!(error = ?error, conversation_id, "saying what the Forge has not got failed");
    }

    Err(world::Unforged::Lacking)
}

fn mergeable(mergeable: Mergeable) -> world::Mergeable {
    match mergeable {
        Mergeable::Cleanly => world::Mergeable::Cleanly,
        Mergeable::Conflicting => world::Mergeable::Conflicting,
        Mergeable::Unknown => world::Mergeable::Unknown,
    }
}
