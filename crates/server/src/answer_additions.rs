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
//! `verkstead_store`'s `set_additions::apply` — once [`undeclared`] and
//! [`readied`] have asked every question the store cannot, and [`submit`] has
//! made the companions' checkouts.
//!
//! **And where the session that asked is waiting on the answer, it is carried
//! on in the wider sandbox**: ended before the Response is stored, and resumed
//! by its harness with a note naming the Set — see [`submit`].

use std::collections::HashMap;

use anyhow::Result;
use verkstead_render::{
    AddableView, Added, AdditionMade, AdditionRemoved, AdditionView, CompanionAddition,
    CompanionMode, CompanionUpgrade, ConversationSteered, Pick, SteerCompanionRefusal,
};
use verkstead_schema::Response;

use crate::answer_files::{Fixed, Standing, asks, standing};
use crate::drivers::Driving;
use crate::steering::{Additions, Alongside, Upgrades};
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

/// What asking git about a Set's pending companions came to, before anything
/// is stored.
pub(crate) enum Readied {
    /// Every checkout planned and none made: [`submit`] makes them.
    Ready(Alongside),

    /// A Repo picked on the Set is no longer registered, by its name.
    RepoGone(String),

    /// A Repo cannot go in, or be opened up, as it was picked: by name, with
    /// the Steer's own reason.
    Refused {
        repo: String,
        why: SteerCompanionRefusal,
    },
}

/// Every question the Set's pending companions and opened-up companions turn
/// on, asked before the Response is stored: whether each Repo is still
/// registered, the Steer's questions of the record — see
/// [`crate::steering::joining`] and [`crate::steering::opening`] — and its
/// questions of git, fetching, resolving the base and finding the read-write
/// branch free — see [`Alongside::plan`]. Nothing is made (ADR-0028, *Every
/// question before anything is ended*).
///
/// A Conversation with nothing checked out yet is asked the record's questions
/// and none of git's: its grill start checks every companion out.
pub(crate) async fn readied(state: &AppState, set_id: i64) -> Result<Readied> {
    let conversation = match standing(&state.pool, set_id).await? {
        Standing::Waiting(conversation) => conversation,
        Standing::Fixed(_) => return Ok(Readied::Ready(Alongside::none())),
    };

    let mut joining = Vec::new();
    let mut opening = Vec::new();

    for added in store::set_additions(&state.pool, set_id).await? {
        if added.state != store::AdditionState::Pending {
            continue;
        }

        let repo_id = match &added.addition {
            store::Addition::Companion { repo_id, .. }
            | store::Addition::OpenedUp { repo_id, .. } => *repo_id,
            store::Addition::Server { .. }
            | store::Addition::Device { .. }
            | store::Addition::Unmatched { .. } => continue,
        };

        if store::registered_repo(&state.pool, repo_id)
            .await?
            .is_none()
        {
            let name = store::load_repo(&state.pool, repo_id)
                .await?
                .map_or_else(|| format!("repo {repo_id}"), |repo| repo.name);

            return Ok(Readied::RepoGone(name));
        }

        match added.addition {
            store::Addition::Companion {
                repo_id,
                mode,
                base,
                branch,
            } => joining.push(CompanionAddition {
                repo_id,
                mode: crate::steering::reaching(mode),
                base_ref: base,
                branch: branch.unwrap_or_default(),
            }),
            store::Addition::OpenedUp {
                repo_id,
                base,
                branch,
            } => opening.push((
                CompanionUpgrade {
                    repo_id,
                    branch: branch.unwrap_or_default(),
                },
                base,
            )),
            store::Addition::Server { .. }
            | store::Addition::Device { .. }
            | store::Addition::Unmatched { .. } => {}
        }
    }

    if joining.is_empty() && opening.is_empty() {
        return Ok(Readied::Ready(Alongside::none()));
    }

    let Some(conversation) = store::load_conversation(&state.pool, conversation).await? else {
        return Ok(Readied::Ready(Alongside::none()));
    };

    let added = match crate::steering::joining(state, &conversation, &joining).await? {
        Additions::Ready(added) => added,
        Additions::Refused(refusal) => return refused(refusal),
    };

    let upgrades: Vec<CompanionUpgrade> = opening.iter().map(|(up, _)| up.clone()).collect();
    let mut opened = match crate::steering::opening(state, &conversation, &upgrades).await? {
        Upgrades::Ready(opened) => opened,
        Upgrades::Refused(refusal) => return refused(refusal),
    };

    // The base the card named, where it named one: the branch is cut from
    // there rather than from the base the companion was first checked out off.
    for companion in &mut opened {
        if let Some((_, Some(base))) = opening
            .iter()
            .find(|(up, _)| up.repo_id == companion.repo.id)
        {
            companion.base_ref = Some(base.clone());
        }
    }

    if conversation.worktree.is_none() {
        return Ok(Readied::Ready(Alongside::none()));
    }

    match Alongside::plan(state, &conversation, added, opened).await? {
        Ok(alongside) => Ok(Readied::Ready(alongside)),
        Err(refusal) => refused(refusal),
    }
}

/// A steer's refusal, said of an Answer.
fn refused(refusal: ConversationSteered) -> Result<Readied> {
    match refusal {
        ConversationSteered::Companion { repo, why } => Ok(Readied::Refused { repo, why }),
        // Asked of the registry above, so only a Repo unregistered in between.
        ConversationSteered::NoSuchCompanionRepo => Ok(Readied::RepoGone("a repo".to_owned())),
        other => anyhow::bail!("a companion check refused an Answer with {other:?}"),
    }
}

/// What storing a Response with its companions came to.
pub(crate) enum Answering {
    /// The store's own answer; the checkouts are recorded where it is
    /// [`store::Submission::Accepted`], and taken back where it is not.
    Stored(store::Submission),

    /// Git would not make a checkout: nothing was stored, and nothing is left
    /// made.
    Refused {
        repo: String,
        why: SteerCompanionRefusal,
    },
}

/// Make the checkouts [`readied`] planned, then store the Response — which
/// puts the companions on the Conversation and records where each went, in one
/// transaction — and only then remove the detached checkouts the opened-up
/// ones replace. A Response the store refuses takes the new checkouts back
/// with it.
///
/// **And where a session is waiting on this very Set and what it adds changes
/// the sandbox, that session is carried on in a wider one** (ADR-0028) — see
/// [`widening`], which is who and when. The order is the Steer's: the Response
/// is asked whether it answers the Set before anything is ended, the checkouts
/// are made while the session still runs, and then the session is ended where
/// it stands, *before* the Response is stored, so the wait it is idling in
/// never prints the Answers. What is launched after is the harness's own
/// resume where it can be, primed to read them once — see [`carry_on`].
///
/// Everywhere else nothing running is ended: a session keeps the binds it
/// started with, and a companion reaches the next launch, as one a steer adds
/// does. The detached directory an opened-up companion replaces goes from
/// under a running session as the Steer's make removes it — see
/// [`crate::steering`].
pub(crate) async fn submit(
    state: &AppState,
    set_id: i64,
    alongside: Alongside,
    response: &Response,
) -> Result<Answering> {
    let widening = widening(state, set_id, response).await?;

    // The last question before anything is ended: a Response that leaves a
    // Question unanswered is refused with the session still waiting.
    if widening.is_some()
        && let Some(refused) = store::unresolved(&state.pool, set_id, response).await?
    {
        return Ok(Answering::Stored(refused));
    }

    // From the making to the record naming it, as a steer holds it: a directory
    // made and not yet recorded is one the orphan sweep would read as nobody's.
    let making = state.checkouts.lock().await;

    let (alongside, checkouts) = match alongside.check_out().await? {
        Ok(made) => made,
        Err(ConversationSteered::Companion { repo, why }) => {
            return Ok(Answering::Refused { repo, why });
        }
        Err(other) => anyhow::bail!("checking out an Answer's companions refused with {other:?}"),
    };

    let driving = match widening {
        Some(conversation_id) => Some((
            conversation_id,
            ending(state, conversation_id, set_id).await,
        )),
        None => None,
    };

    let stored = store::submit_response_with_checkouts(
        &state.pool,
        &state.settlements,
        set_id,
        response,
        &checkouts,
    )
    .await;

    let submission = match stored {
        Ok(submission) => submission,
        Err(error) => {
            alongside.unmake().await?;
            drop(making);
            carry_on(state, driving, set_id);
            return Err(error);
        }
    };

    match submission {
        store::Submission::Accepted(_) => {
            alongside.retire().await?;

            if driving.is_some() {
                store::carried_on(&state.pool, set_id).await?;
            }
        }
        _ => alongside.unmake().await?,
    }

    drop(making);

    // Whatever the store said: the session has been ended, and the one
    // waiting on it is owed its carrying-on either way.
    carry_on(state, driving, set_id);

    Ok(Answering::Stored(submission))
}

/// The Conversation whose session is waiting on Set `set_id` and is to be
/// carried on in a wider sandbox when `response` is stored, or `None` where
/// answering it ends nothing (ADR-0028, *Only what changes the sandbox
/// restarts the agent*, *Only the session waiting on that Set*).
///
/// **What changes the sandbox**: a pending Companion, a Companion opened up, or
/// an MCP server. A device tick is checked live by `verkstead transfer`, and a
/// file already reaches the running session, so a Set adding only those ends
/// nothing.
///
/// **And a session waiting on that very Set**: one asked the ordinary way —
/// blocking or store-and-nudge, never a Deferred Ask, which nobody idles on —
/// still open, with a session running that was already running when the Set
/// was asked. A relaunch locks the Sets it orphans, so an open Set and a
/// session older than it is that session's own question; a Set whose asker has
/// gone with nothing started since has no session running at all. Or a Set that
/// crossed on a Transfer, which the arrival left open and whose asker the
/// session the arrival started carries on (ADR-0028, *Pending additions cross
/// with the record*).
///
/// **And a state whose relaunch is that same session again**: a grilling, a run,
/// a follow-up or an investigation, each of which Resume starts again by
/// launching a session. A wrap-up's watchers send sessions for reasons of their
/// own, so a Set asked in one is left to the next launch. So is a Response that
/// picks a direction on a proposal — the work moves on from there in a session
/// of its own — and a Conversation that is stopped, which no launch passes.
async fn widening(state: &AppState, set_id: i64, response: &Response) -> Result<Option<i64>> {
    if response.direction.is_some() || store::settlement(&state.pool, set_id).await?.is_some() {
        return Ok(None);
    }

    let restarts = store::set_additions(&state.pool, set_id)
        .await?
        .iter()
        .any(|added| {
            added.state == store::AdditionState::Pending
                && !matches!(added.addition, store::Addition::Device { .. })
        });

    if !restarts || store::asked_as(&state.pool, set_id).await? == store::Ask::Deferred {
        return Ok(None);
    }

    let Some((conversation_id, asked_at)) = store::opened_at(&state.pool, set_id).await? else {
        return Ok(None);
    };

    let Some(session) = state.sessions.following(conversation_id) else {
        return Ok(None);
    };

    // Or one started after it on this device, where the Set crossed with the
    // record: an arrival leaves the Sets it brought open and its session is
    // the asker's carried on, told to wait on them again.
    if session.event_id > asked_at
        && !store::sets_as_they_landed(&state.pool, conversation_id)
            .await?
            .values()
            .any(|landed| *landed == set_id)
    {
        return Ok(None);
    }

    if store::stopped(&state.pool, conversation_id)
        .await?
        .is_some()
    {
        return Ok(None);
    }

    Ok(match store::state(&state.pool, conversation_id).await? {
        Some(
            store::Lifecycle::Grilling
            | store::Lifecycle::Implementing
            | store::Lifecycle::FollowUp
            | store::Lifecycle::Investigating,
        ) => Some(conversation_id),
        _ => None,
    })
}

/// End the session waiting on Set `set_id` where it stands, having written down
/// that the next launch carries its conversation on.
///
/// The registration is taken first and handed to the launch, so there is no
/// moment where the Conversation reads as undriven and a stall sweep stops it.
/// Ended by Verkstead, so whatever was following the session reads the ending
/// as deliberate — see [`crate::sessions::Ended::on_purpose`] — exactly as a
/// steer's Interrupt is read.
async fn ending(state: &AppState, conversation_id: i64, set_id: i64) -> Driving {
    let driving = state.drivers.driving(conversation_id);

    // The newest session is the one waiting, so it is the one to resume — the
    // arrival's own reading, with the Set beside it to say why.
    let written = match store::the_last_session(&state.pool, conversation_id).await {
        Ok(Some(continued)) => {
            let continued = store::Continued {
                widened_by: Some(set_id),
                ..continued
            };

            match store::continue_on_arrival(&state.pool, conversation_id, &continued).await {
                Ok(()) => true,
                Err(error) => {
                    tracing::error!(error = ?error, conversation_id, "recording the conversation an answer is carrying on failed, so the session started for it will be re-primed instead");
                    false
                }
            }
        }
        Ok(None) => false,
        Err(error) => {
            tracing::error!(error = ?error, conversation_id, "reading the session an answer is carrying on failed, so the session started for it will be re-primed instead");
            false
        }
    };

    if !written {
        crate::carrying::instead(
            &state.pool,
            &state.nudges,
            conversation_id,
            crate::carrying::Occasion::Widened(set_id),
            &crate::carrying::Instead::NoSession,
        )
        .await;
    }

    state.sessions.end(conversation_id).await;

    tracing::info!(
        conversation_id,
        set_id,
        "answering a Question Set added to the sandbox, so the session waiting on it was ended \
         to be carried on in a wider one",
    );

    driving
}

/// Launch what carries on the session [`ending`] ended, where one was.
fn carry_on(state: &AppState, driving: Option<(i64, Driving)>, set_id: i64) {
    if let Some((conversation_id, driving)) = driving {
        tokio::spawn(crate::resume::widened(
            state.clone(),
            conversation_id,
            set_id,
            driving,
        ));
    }
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
    let companions = store::companions(&state.pool, conversation_id).await?;
    let carried = store::carried_sets(&state.pool, conversation_id)
        .await?
        .contains(&set_id);

    for added in applied {
        let said = said(&added.addition, &naming, agent, &companions, carried);

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
/// with. A device is named with its id and the command that uses it. A
/// companion is named with the directory it is checked out in and its mode, as
/// the prompt's own listing names one — see [`crate::skills::alongside`] — and
/// reaches the next launch for the same reason a server does.
///
/// **Unless the answer carried the session on** — `carried` — in which case the
/// session reading this is the one launched with them, and says so.
fn said(
    addition: &store::Addition,
    naming: &Naming,
    agent: Option<store::AgentType>,
    companions: &[store::Companion],
    carried: bool,
) -> String {
    const CARRIED: &str =
        "Verkstead carried this conversation on in a session started with it, which is this one.";

    match addition {
        // Never applied, being struck as it crossed: said for completeness.
        store::Addition::Unmatched { name } => format!(
            "The repo `{name}`, which no repo on this device matched when the work was moved \
             here, so it was not added."
        ),
        store::Addition::Server { name } => {
            let tools = agent.map_or_else(String::new, |agent| {
                format!(
                    ", whose tools are named `{}`",
                    crate::skills::tools_named(agent).replace("<server>", name)
                )
            });

            let reach = match carried {
                true => CARRIED,
                false => {
                    "It is attached to this Conversation from the next session launched; this \
                     one was started without it."
                }
            };

            format!("The MCP server `{name}`{tools}. {reach}")
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
        store::Addition::Companion {
            repo_id,
            mode,
            branch,
            ..
        } => {
            let reach = match (mode, branch) {
                (store::CompanionMode::ReadWrite, Some(branch)) => {
                    format!("read-write, on branch `{branch}`")
                }
                (store::CompanionMode::ReadWrite, None) => "read-write".to_owned(),
                (store::CompanionMode::ReadOnly, _) => "read-only".to_owned(),
            };

            format!(
                "The repo `{}`{}, {reach}. {}",
                called(naming, *repo_id),
                checked_out(companions, *repo_id),
                match carried {
                    true => CARRIED,
                    false => {
                        "It is bound from the next session launched; this one was started \
                         without it."
                    }
                },
            )
        }
        store::Addition::OpenedUp {
            repo_id, branch, ..
        } => {
            let on = branch
                .as_deref()
                .map_or_else(String::new, |branch| format!(", on branch `{branch}`"));

            format!(
                "The repo `{}`, opened up to read-write{on}{}. {}",
                called(naming, *repo_id),
                checked_out(companions, *repo_id),
                match carried {
                    true => CARRIED,
                    false => {
                        "It is bound so from the next session launched; this one was started \
                         with it read-only."
                    }
                },
            )
        }
    }
}

/// What the note a widened session is resumed with names: the Set's title,
/// where its body can be read, and each thing its answer added, in a few words
/// — the Answers themselves say the rest under `added`. See
/// [`crate::skills::widened`].
///
/// Read with the pool alone, being asked from inside a launch. A read that
/// fails costs the note a line rather than the resume: said in the log.
pub(crate) async fn widened_with(
    pool: &sqlx::SqlitePool,
    set_id: i64,
) -> (Option<String>, Vec<String>) {
    let title = match store::load_set(pool, set_id).await {
        Ok(Some(stored)) => stored
            .set
            .readable(set_id)
            .ok()
            .map(|set| set.title.clone()),
        Ok(None) => None,
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading the title of the Set a widened session is told about failed");
            None
        }
    };

    let (added, repos) = match tokio::try_join!(
        store::set_additions(pool, set_id),
        store::registered_repos(pool),
    ) {
        Ok(read) => read,
        Err(error) => {
            tracing::error!(error = ?error, set_id, "reading what a Set added, for the session it widened, failed");
            return (title, Vec::new());
        }
    };

    let repo = |id: i64| {
        repos
            .iter()
            .find(|repo| repo.id == id)
            .map_or_else(|| format!("registered as {id}"), |repo| repo.name.clone())
    };

    let added = added
        .into_iter()
        .filter(|added| added.state == store::AdditionState::Applied)
        .map(|added| match added.addition {
            store::Addition::Server { name } => format!("the MCP server `{name}`"),
            store::Addition::Companion { repo_id, mode, .. } => format!(
                "the repo `{}`, {}",
                repo(repo_id),
                match mode {
                    store::CompanionMode::ReadOnly => "read-only",
                    store::CompanionMode::ReadWrite => "read-write",
                },
            ),
            store::Addition::OpenedUp { repo_id, .. } => {
                format!("the repo `{}`, opened up to read-write", repo(repo_id))
            }
            store::Addition::Device { device } => {
                format!("leave to move the work to the device `{device}`")
            }
            store::Addition::Unmatched { name } => format!("the repo `{name}`, not added"),
        })
        .collect();

    (title, added)
}

/// A Repo by its registered name, or its id where it has none now.
fn called(naming: &Naming, repo_id: i64) -> String {
    naming
        .repo(repo_id)
        .unwrap_or_else(|| format!("registered as {repo_id}"))
}

/// Where a companion is checked out, said as a clause, or nothing where it is
/// not yet.
fn checked_out(companions: &[store::Companion], repo_id: i64) -> String {
    companions
        .iter()
        .find(|companion| companion.repo.id == repo_id)
        .and_then(|companion| companion.worktree.as_ref())
        .map_or_else(String::new, |worktree| {
            format!(", checked out at `{}`", worktree.display())
        })
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
            store::Addition::Unmatched { name } => Added::Unmatched { name },
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
