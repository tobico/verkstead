//! What a device lacks before a Conversation can be moved onto it (ADR-0020,
//! *Transfer*).
//!
//! **A reading rather than a press.** The Transfer dialog draws it the moment a
//! device is picked, and **Go** is refused while it holds anything — by name,
//! because each finding is an errand somewhere rather than a count: a repository
//! to open over there, a harness to install over there, or a machine to go and
//! wake.
//!
//! **Asked of the far end, with one exception.** Whether one of that device's
//! Repos *is* this repository is settled by [`crate::matching`], which runs on
//! the end that is going to act on the answer: the far end sends its registry
//! and this end applies the rule, which is what stops two Verksteads one version
//! apart disagreeing about which repository the work is in. Everything else only
//! the far end can look at, so it answers about itself — see
//! [`crate::peer::harnesses`].
//!
//! **Two questions, both of them about one machine.** The registry and the
//! harnesses are asked for in turn over a link that is already open, and either
//! of them coming back unanswered means the same thing: the machine is not
//! there. So it is one finding and the whole of the preflight, rather than a
//! list of everything a silent device failed to confirm — *unreachable* and *no
//! match* are two different things to say, one sending the human to **Open
//! repo** on a machine that may already have the repository and the other
//! sending them to the machine.
//!
//! **And the Conversation's repositories are all of them**: the Repo the work is
//! in, and every Companion beside it. A Companion travels the same way the
//! branch does, so a Companion with nowhere to land is as much a refusal as the
//! repository itself — said as a different sentence, being a different thing to
//! go and put right.

use verkstead_render::{HarnessThere, Lacking, PairingRole, Preflight};

use crate::device::Devices;
use crate::peer::harnesses::HARNESSES;
use crate::relaying::{self, Refusal};
use crate::store;

/// The most a device's harnesses may be before the answer is dropped: **eight
/// kilobytes**.
///
/// Four rows of a word and a boolean, so this is a thousand times what an answer
/// weighs. What it is for is the case every bound across a link is for: a
/// machine on the far end that answers and then writes without stopping — see
/// [`crate::matching::MOST_THE_REPOS_ARE`], which is the same bound one reading
/// along.
const MOST_THE_HARNESSES_ARE: usize = 8 * 1024;

/// What `device` lacks before `conversation` could be moved onto it.
///
/// `Err` is this device's own trouble — a store it could not read, a cluster it
/// is not part of. A device that did not answer is not one of those: it is a
/// finding about that machine, and it comes back as one.
pub async fn of(
    devices: Option<&Devices>,
    pool: &sqlx::SqlitePool,
    conversation: &store::Conversation,
    device: &str,
) -> Result<Preflight, Refusal> {
    let Some(devices) = devices else {
        return Err(Refusal::ours(
            "this server holds no device identity to ask a member through".to_owned(),
        ));
    };

    // The machine by the name the human gave it, which is what every finding
    // below is said against — and what the one about the machine itself is.
    let named = relaying::called(Some(devices), device).await;

    let companions = store::companions(pool, conversation.id)
        .await
        .map_err(|why| {
            Refusal::ours(format!(
                "this Conversation's companions could not be read: {why:#}"
            ))
        })?;

    // The Conversation's own repository first, then each Companion in the order
    // the record has them: the same order the findings are drawn in.
    let repos: Vec<store::Repo> = std::iter::once(conversation.repo.clone())
        .chain(companions.into_iter().map(|companion| companion.repo))
        .collect();

    let asked = ask(devices, device, &repos).await;

    let (matched, harnesses) = match asked {
        Ok(answered) => answered,

        // The machine said nothing, which is the whole of what there is to say
        // about it. Anything else is this device's own trouble and is reported
        // as a failure rather than dressed up as a finding.
        Err(refusal) if refusal.from_them() => {
            return Ok(Preflight {
                device: named,
                lacks: vec![Lacking::Unreachable],
            });
        }

        Err(refusal) => return Err(refusal),
    };

    let mut lacks = Vec::new();

    for (at, (repo, matched)) in repos.iter().zip(matched).enumerate() {
        if matched.is_none() {
            lacks.push(Lacking::Repo {
                name: repo.name.clone(),
                // The first of the list is the Conversation's own; everything
                // after it came off the companions.
                companion: at > 0,
            });
        }
    }

    for (role, pairing) in pairings(conversation) {
        let agent_type = pairing.profile.agent_type();

        // Absent from the answer is a harness the far end has never heard of,
        // which is a Verkstead older than this one: read as *there* rather than
        // as missing, a move refused over a probe nobody made being worse than
        // one made and found wanting.
        let there = harnesses
            .iter()
            .find(|harness| harness.agent_type == crate::profiles::agent_type(agent_type))
            .is_none_or(|harness| harness.there);

        if !there {
            lacks.push(Lacking::Harness {
                role,
                profile: pairing.profile.name.clone(),
                agent_type: crate::profiles::agent_type(agent_type),
            });
        }
    }

    Ok(Preflight {
        device: named,
        lacks,
    })
}

/// Both questions put to the machine, in the order a move would meet them.
///
/// Its own function so that either of them going unanswered is one arm above
/// rather than two: what the caller does with *the machine is not there* is the
/// same whichever of the two found it out.
async fn ask(
    devices: &Devices,
    device: &str,
    repos: &[store::Repo],
) -> Result<(Vec<Option<verkstead_render::RepoAcross>>, Vec<HarnessThere>), Refusal> {
    let matched = crate::matching::each_across(devices, device, repos).await?;

    let harnesses: Vec<HarnessThere> = relaying::word_from(
        Some(devices),
        device,
        relaying::asking(HARNESSES.to_owned()),
        MOST_THE_HARNESSES_ARE,
    )
    .await?;

    Ok((matched, harnesses))
}

/// Every Pairing this Conversation names, in the order the work goes through
/// them.
///
/// The three roles, and only the ones something was picked for: a role picked
/// away runs no session at all, and a role nothing has been picked for yet is
/// not a harness anybody is asking the far end about.
fn pairings(conversation: &store::Conversation) -> Vec<(PairingRole, &store::Pairing)> {
    [
        (
            PairingRole::Grilling,
            conversation.grilling_pairing.pairing(),
        ),
        (
            PairingRole::Implementation,
            conversation.implementation_pairing.as_ref(),
        ),
        (PairingRole::Review, conversation.review_pairing.pairing()),
    ]
    .into_iter()
    .filter_map(|(role, pairing)| pairing.map(|pairing| (role, pairing)))
    .collect()
}
