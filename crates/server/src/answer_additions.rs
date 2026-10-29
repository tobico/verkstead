//! What the human picks on a Question Set's sheet to add to the work: the
//! Attach menu's rows past *Attach file…*, recorded on the Set under the
//! Question they were picked at, or under none at the comment box (ADR-0028).
//!
//! **The Answer's files said again for a reference rather than bytes** — see
//! [`crate::answer_files`], whose refusals these share: a Set answered or locked
//! unanswered, and a Closed Conversation, fix what was picked exactly as they
//! fix what was attached, and a label has to be one the Set asks.
//!
//! **Recorded rather than applied.** Nothing here touches the Conversation: its
//! own servers are as they were, and a launch after a pick is written with the
//! same Built Root. What a pick is for is the Submit, which applies the Set's
//! pending additions in the transaction that stores the Response — see
//! `verkstead_store`'s `set_additions::apply` — once [`undeclared`] has asked
//! the one question the store cannot.

use std::collections::HashMap;

use anyhow::Result;
use verkstead_render::{
    AddableView, Added, AdditionMade, AdditionRemoved, AdditionView, CompanionMode, Pick,
};
use verkstead_schema::Response;

use crate::answer_files::{Fixed, Standing, asks, standing};
use crate::{AppState, store};

/// Record a pick on a waiting Set, pending.
pub(crate) async fn add(
    state: &AppState,
    set_id: i64,
    label: Option<&str>,
    pick: &Pick,
) -> Result<AdditionMade> {
    let conversation = match standing(&state.pool, set_id).await? {
        Standing::Waiting(conversation) => conversation,
        Standing::Fixed(refusal) => return Ok(adding(refusal)),
    };

    if let Some(label) = label
        && !asks(&state.pool, set_id, label).await?
    {
        return Ok(AdditionMade::NoSuchLabel);
    }

    let addition = match pick {
        Pick::Server { name } => {
            if !crate::conversations::declared_servers(state).contains(name) {
                return Ok(AdditionMade::NoSuchServer);
            }

            if store::mcp_servers(&state.pool, conversation)
                .await?
                .contains(name)
            {
                return Ok(AdditionMade::AlreadyOn);
            }

            store::Addition::Server { name: name.clone() }
        }

        // A Repo registered here, and neither the work's own nor alongside it
        // already: the menu offers no other, and a press arriving for one is a
        // registration or a steer that landed between the drawing and the
        // press.
        Pick::Companion {
            repo_id,
            mode,
            base_ref,
            branch,
        } => {
            if store::registered_repo(&state.pool, *repo_id)
                .await?
                .is_none()
            {
                return Ok(AdditionMade::NoSuchRepo);
            }

            if on_it(state, conversation, *repo_id).await?.is_some() {
                return Ok(AdditionMade::AlreadyOn);
            }

            let mode = match mode {
                CompanionMode::ReadOnly => store::CompanionMode::ReadOnly,
                CompanionMode::ReadWrite => store::CompanionMode::ReadWrite,
            };

            store::Addition::Companion {
                repo_id: *repo_id,
                // A read-only checkout is detached and takes no branch, so a
                // name sent with one is a name for a branch nobody will cut.
                branch: match mode {
                    store::CompanionMode::ReadOnly => None,
                    store::CompanionMode::ReadWrite => named(branch),
                },
                mode,
                base: named(base_ref),
            }
        }

        // A Companion of the work held read-only: a read-write one is as open
        // as one gets.
        Pick::OpenedUp {
            repo_id,
            base_ref,
            branch,
        } => match on_it(state, conversation, *repo_id).await? {
            Some(Some(store::CompanionMode::ReadOnly)) => store::Addition::OpenedUp {
                repo_id: *repo_id,
                base: named(base_ref),
                branch: named(branch),
            },
            Some(Some(store::CompanionMode::ReadWrite)) => return Ok(AdditionMade::AlreadyOn),
            Some(None) | None => return Ok(AdditionMade::NotACompanion),
        },

        // A device the agent could not already move the work to, asked the way
        // a tick on the Transfer dialog is — see [`crate::transfers::permit`].
        Pick::Device { device } => {
            let Some(devices) = state.devices.as_ref() else {
                return Ok(AdditionMade::NotAMember);
            };

            let born = store::birth(&state.pool, conversation).await?;
            if born.is_some_and(|born| &born.device == device)
                || store::permitted_devices(&state.pool, conversation)
                    .await?
                    .contains(device)
            {
                return Ok(AdditionMade::AlreadyOn);
            }

            if device == devices.id() {
                return Ok(AdditionMade::WorkIsHere);
            }

            let members = devices.membership().rows().await?;
            if !members.iter().any(|member| &member.device == device) {
                return Ok(AdditionMade::NotAMember);
            }

            store::Addition::Device {
                device: device.clone(),
            }
        }
    };

    Ok(
        match store::add_to_set(&state.pool, conversation, set_id, label, &addition).await? {
            store::AddingToSet::Added(added) => AdditionMade::Added {
                addition: viewed(added, &Naming::read(state).await?),
            },
            store::AddingToSet::Answered => AdditionMade::Answered,
            store::AddingToSet::Locked => AdditionMade::Locked,
        },
    )
}

/// And take a pick off again, which is the × on its chip.
///
/// Nothing is asked about the declarations, for the reason the composer's ×
/// asks nothing: the chip most likely to be taken off is the one whose
/// declaration has gone.
pub(crate) async fn remove(state: &AppState, set_id: i64, id: i64) -> Result<AdditionRemoved> {
    if let Standing::Fixed(refusal) = standing(&state.pool, set_id).await? {
        return Ok(removing(refusal));
    }

    Ok(match store::take_off_set(&state.pool, set_id, id).await? {
        store::TakingOff::TakenOff => AdditionRemoved::Removed,
        store::TakingOff::Answered => AdditionRemoved::Answered,
        store::TakingOff::Locked => AdditionRemoved::Locked,
    })
}

/// Everything picked on one Set, in the shape its page draws it.
pub(crate) async fn picked(state: &AppState, set_id: i64) -> Result<Vec<AdditionView>> {
    let naming = Naming::read(state).await?;

    Ok(store::set_additions(&state.pool, set_id)
        .await?
        .into_iter()
        .map(|added| viewed(added, &naming))
        .collect())
}

/// The first server picked on a waiting Set that the settings no longer
/// declare, which refuses the Submit by name: the human answered on the
/// strength of what they were adding, so the Answers are not stored without it
/// (ADR-0028, *Every question before anything is ended*).
///
/// Only while the Set waits: one that has settled is the store's to refuse, and
/// a lock leaves its picks pending without that being anything to say.
pub(crate) async fn undeclared(state: &AppState, set_id: i64) -> Result<Option<String>> {
    if store::settlement(&state.pool, set_id).await?.is_some() {
        return Ok(None);
    }

    let declared = crate::conversations::declared_servers(state);

    Ok(store::set_additions(&state.pool, set_id)
        .await?
        .into_iter()
        .find_map(|added| match added.addition {
            store::Addition::Server { name }
                if added.state == store::AdditionState::Pending && !declared.contains(&name) =>
            {
                Some(name)
            }
            _ => None,
        }))
}

/// Put what came with the Answers on the Response as the agent reads it: each
/// Answer's own under it, and what was added at the comment box on the
/// Response itself — see [`verkstead_schema::Answer::added`].
///
/// **On the way out rather than in**, for the files' reason — see
/// [`crate::answer_files::onto`]. Only what was applied: a pick nothing put on
/// the Conversation is nothing the agent has.
pub(crate) async fn onto(
    state: &AppState,
    conversation_id: i64,
    set_id: i64,
    response: &mut Response,
) -> Result<()> {
    let applied: Vec<store::SetAddition> = store::set_additions(&state.pool, set_id)
        .await?
        .into_iter()
        .filter(|added| added.state == store::AdditionState::Applied)
        .collect();

    if applied.is_empty() {
        return Ok(());
    }

    let naming = Naming::read(state).await?;
    let agent = store::newest_agent(&state.pool, conversation_id).await?;

    for added in applied {
        let said = said(&added.addition, &naming, agent);

        match added.label.as_deref() {
            None => response.added.push(said),
            Some(label) => {
                if let Some(answer) = response
                    .answers
                    .iter_mut()
                    .find(|answer| answer.label.trim() == label)
                {
                    answer.added.push(said);
                }
            }
        }
    }

    Ok(())
}

/// One addition in words for the agent: what it is and where it is.
///
/// A server is named with the prefix its tools carry, as the prompt's own
/// listing names them — see [`crate::skills::tools_named`] — and says that it
/// reaches the next launch: a running session keeps the Built Root it started
/// with. A device is named with its id and the command that uses it.
fn said(addition: &store::Addition, naming: &Naming, agent: Option<store::AgentType>) -> String {
    match addition {
        store::Addition::Server { name } => {
            let tools = agent.map_or_else(String::new, |agent| {
                format!(
                    ", whose tools are named `{}`",
                    crate::skills::tools_named(agent).replace("<server>", name)
                )
            });

            format!(
                "The MCP server `{name}`{tools}. It is attached to this Conversation from the \
                 next session launched; this one was started without it."
            )
        }
        store::Addition::Device { device } => {
            let name = naming.device(device);
            let called = name.as_deref().unwrap_or(device);

            format!(
                "The device {} (`{device}`), which this work may now be moved to with \
                 `verkstead transfer {called}`.",
                name.as_deref().unwrap_or("with no name in the cluster now"),
            )
        }
        store::Addition::Companion { repo_id, .. } | store::Addition::OpenedUp { repo_id, .. } => {
            format!(
                "The repo {}.",
                naming
                    .repo(*repo_id)
                    .unwrap_or_else(|| format!("registered as {repo_id}"))
            )
        }
    }
}

/// What each Set of a Conversation added to it, by Set, applied: the line the
/// Timeline draws under a Set's Event.
pub(crate) async fn on_timeline(
    state: &AppState,
    conversation_id: i64,
) -> Result<HashMap<i64, Vec<Added>>> {
    let applied: Vec<store::SetAddition> =
        store::conversation_additions(&state.pool, conversation_id)
            .await?
            .into_iter()
            .filter(|added| added.state == store::AdditionState::Applied)
            .collect();

    let mut by_set: HashMap<i64, Vec<Added>> = HashMap::new();
    if applied.is_empty() {
        return Ok(by_set);
    }

    let naming = Naming::read(state).await?;
    for added in applied {
        let set_id = added.set_id;
        by_set
            .entry(set_id)
            .or_default()
            .push(viewed(added, &naming).added);
    }

    Ok(by_set)
}

/// What a lock, or the Conversation closing, left unapplied on a Set nobody
/// answered: struck, which is what its chips are drawn as on the record.
pub(crate) fn unapplied(additions: Vec<AdditionView>) -> Vec<AdditionView> {
    additions
        .into_iter()
        .map(|addition| match addition.state {
            verkstead_render::AdditionState::Pending => AdditionView {
                state: verkstead_render::AdditionState::Struck,
                ..addition
            },
            _ => addition,
        })
        .collect()
}

/// What the menus offer past the servers, as the Conversation stands now: the
/// Repos it could work alongside, the Companions it could open up, and the
/// devices it could be permitted to move to.
pub(crate) async fn addable(state: &AppState, conversation: i64) -> Result<AddableView> {
    let Some(record) = store::load_conversation(&state.pool, conversation).await? else {
        return Ok(AddableView::default());
    };

    let companions = store::companions(&state.pool, conversation).await?;

    let config = state.settings.config();
    let repos = store::registered_repos(&state.pool)
        .await?
        .into_iter()
        .filter(|repo| {
            repo.id != record.repo.id
                && !companions
                    .iter()
                    .any(|companion| companion.repo.id == repo.id)
        })
        .map(|repo| crate::repos::entry(repo, config.authors()))
        .collect();

    let mut read_only = Vec::new();
    for companion in companions {
        if companion.mode == store::CompanionMode::ReadOnly {
            read_only.push(crate::ui::companion(companion, config.authors()).await?);
        }
    }

    // Every member but the ones the agent may already go to — the drafting
    // device, always, and the ticked — and never this one, which is where the
    // work is.
    let devices = match state.devices.as_ref() {
        None => Vec::new(),
        Some(devices) => {
            let born = store::birth(&state.pool, conversation).await?;
            let permitted = store::permitted_devices(&state.pool, conversation).await?;

            devices
                .membership()
                .rows()
                .await?
                .iter()
                .filter(|member| {
                    member.device != devices.id()
                        && born
                            .as_ref()
                            .is_none_or(|born| born.device != member.device)
                        && !permitted.contains(&member.device)
                })
                .map(crate::device::presenting)
                .collect()
        }
    };

    Ok(AddableView {
        repos,
        read_only,
        devices,
        branch: record.branch,
    })
}

/// Whether a Repo is on the Conversation already: `Some(None)` for its own,
/// `Some(Some(mode))` for a Companion, `None` for neither.
async fn on_it(
    state: &AppState,
    conversation: i64,
    repo_id: i64,
) -> Result<Option<Option<store::CompanionMode>>> {
    let Some(record) = store::load_conversation(&state.pool, conversation).await? else {
        return Ok(None);
    };

    if record.repo.id == repo_id {
        return Ok(Some(None));
    }

    Ok(store::companions(&state.pool, conversation)
        .await?
        .into_iter()
        .find(|companion| companion.repo.id == repo_id)
        .map(|companion| Some(companion.mode)))
}

/// A base or branch name as the card sends it, with the empty string being the
/// rule — the default branch, or *mirroring* — which the record holds as none.
fn named(name: &Option<String>) -> Option<String> {
    name.as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
}

/// What a chip names each thing by, read once for a list of them: the declared
/// servers, the registered Repos and the cluster's devices.
struct Naming {
    declared: Vec<String>,
    repos: Vec<(i64, String)>,
    devices: Vec<(String, String)>,
}

impl Naming {
    async fn read(state: &AppState) -> Result<Self> {
        let repos = store::registered_repos(&state.pool)
            .await?
            .into_iter()
            .map(|repo| (repo.id, repo.name))
            .collect();

        let devices = match state.devices.as_ref() {
            None => Vec::new(),
            Some(devices) => devices
                .membership()
                .rows()
                .await?
                .into_iter()
                .map(|member| (member.device, member.name))
                .collect(),
        };

        Ok(Self {
            declared: crate::conversations::declared_servers(state),
            repos,
            devices,
        })
    }

    fn repo(&self, id: i64) -> Option<String> {
        self.repos
            .iter()
            .find(|(repo, _)| *repo == id)
            .map(|(_, name)| name.clone())
    }

    fn device(&self, device: &str) -> Option<String> {
        self.devices
            .iter()
            .find(|(id, _)| id == device)
            .map(|(_, name)| name.clone())
    }
}

fn viewed(added: store::SetAddition, naming: &Naming) -> AdditionView {
    AdditionView {
        id: added.id,
        label: added.label,
        added: match added.addition {
            store::Addition::Server { name } => Added::Server {
                declared: naming.declared.contains(&name),
                name,
            },
            store::Addition::Companion {
                repo_id,
                mode,
                base,
                branch,
            } => Added::Companion {
                repo_id,
                name: naming.repo(repo_id),
                mode: match mode {
                    store::CompanionMode::ReadOnly => CompanionMode::ReadOnly,
                    store::CompanionMode::ReadWrite => CompanionMode::ReadWrite,
                },
                base_ref: base,
                branch,
            },
            store::Addition::OpenedUp {
                repo_id,
                base,
                branch,
            } => Added::OpenedUp {
                repo_id,
                name: naming.repo(repo_id),
                base_ref: base,
                branch,
            },
            store::Addition::Device { device } => Added::Device {
                name: naming.device(&device),
                device,
            },
        },
        state: match added.state {
            store::AdditionState::Pending => verkstead_render::AdditionState::Pending,
            store::AdditionState::Applied => verkstead_render::AdditionState::Applied,
            store::AdditionState::Struck => verkstead_render::AdditionState::Struck,
        },
    }
}

fn adding(refusal: Fixed) -> AdditionMade {
    match refusal {
        Fixed::NoSuchSet => AdditionMade::NoSuchSet,
        Fixed::Answered => AdditionMade::Answered,
        Fixed::Locked => AdditionMade::Locked,
        Fixed::Closed => AdditionMade::Closed,
    }
}

fn removing(refusal: Fixed) -> AdditionRemoved {
    match refusal {
        Fixed::NoSuchSet => AdditionRemoved::NoSuchSet,
        Fixed::Answered => AdditionRemoved::Answered,
        Fixed::Locked => AdditionRemoved::Locked,
        Fixed::Closed => AdditionRemoved::Closed,
    }
}
