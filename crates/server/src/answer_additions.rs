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
//! pending additions in the transaction that stores the Response.

use anyhow::Result;
use sqlx::SqlitePool;
use verkstead_render::{Added, AdditionMade, AdditionRemoved, AdditionView, Pick};

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
    };

    Ok(
        match store::add_to_set(&state.pool, conversation, set_id, label, &addition).await? {
            store::AddingToSet::Added(added) => AdditionMade::Added {
                addition: viewed(added, &crate::conversations::declared_servers(state)),
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
pub(crate) async fn picked(
    pool: &SqlitePool,
    declared: &[String],
    set_id: i64,
) -> Result<Vec<AdditionView>> {
    Ok(store::set_additions(pool, set_id)
        .await?
        .into_iter()
        .map(|added| viewed(added, declared))
        .collect())
}

fn viewed(added: store::SetAddition, declared: &[String]) -> AdditionView {
    AdditionView {
        id: added.id,
        label: added.label,
        added: match added.addition {
            store::Addition::Server { name } => Added::Server {
                declared: declared.contains(&name),
                name,
            },
            store::Addition::Companion {
                repo_id,
                mode,
                base,
                branch,
            } => Added::Companion {
                repo_id,
                mode: match mode {
                    store::CompanionMode::ReadOnly => verkstead_render::CompanionMode::ReadOnly,
                    store::CompanionMode::ReadWrite => verkstead_render::CompanionMode::ReadWrite,
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
                base_ref: base,
                branch,
            },
            store::Addition::Device { device } => Added::Device { device },
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
