//! The platform default: what a Pairing picker on a Repo nothing has grilled is
//! filled with when there is no last start anywhere to copy either.
//!
//! The one place the server knows anything about which models are which. The
//! viewer has a catalog of names to draw — see `web/src/models.ts` — and the
//! server has never needed one, because every model a session runs on is one a
//! human picked off a Profile's own list. A default is the exception: something
//! has to say that Fable 5 is what interviews and Opus 5 is what builds, and
//! that is a fact about the harnesses rather than about any saved account, so
//! it is written down here as a table rather than asked of a Profile.
//!
//! Frontier for the grilling and the value tier for everything after it, which
//! is the split the work itself has: the interview is where a wrong reading is
//! most expensive, and the build and the review are where the sessions run
//! longest. Review takes the implementation model on every harness.
//!
//! **One candidate and no hunting.** A default is picked by rule — the first
//! harness in [`HARNESSES`] with a Profile saved, that harness's unnamed Profile
//! or else its earliest, the table's model or else the first that Profile lists
//! — and handed back whole, to be judged by the same reading a remembered
//! Pairing gets. A candidate that fails the judging leaves its picker empty; it
//! is not a reason to go looking through the other Profiles for one that would
//! pass, which would make the default depend on which accounts happen to be
//! broken today.

use crate::store::{self, AgentType, Picked, Profile, Role};

/// The harnesses in the order a default prefers them, most preferred first.
///
/// Fixed rather than read off anything, so that saving a second harness's
/// Profile does not quietly move every fresh Repo's default on to it.
const HARNESSES: [AgentType; 4] = [
    AgentType::Claude,
    AgentType::Codex,
    AgentType::Grok,
    AgentType::OpenCode,
];

/// The model a harness's default Pairing runs for a role.
///
/// Ids as a Profile lists them, since a Profile's list is what the default is
/// matched against. Codex has one model it is known to run, so both tiers are
/// it; Grok 4.6 is Grok Build's value tier because the older 4.5 reads as older
/// rather than cheaper.
fn model(harness: AgentType, role: Role) -> &'static str {
    match (harness, role) {
        (AgentType::Claude, Role::Grilling) => "claude-fable-5",
        (AgentType::Claude, Role::Implementation | Role::Review) => "claude-opus-5",
        (AgentType::Codex, _) => "gpt-5-codex",
        (AgentType::Grok, _) => "grok-4.6",
        (AgentType::OpenCode, Role::Grilling) => "opencode/gpt-5.1-codex",
        (AgentType::OpenCode, Role::Implementation | Role::Review) => "minimax/minimax-m2.1",
    }
}

/// The platform default for `role`, out of the Profiles saved — or
/// [`Picked::Nothing`] where there is no Profile to run one under.
///
/// Unjudged: whether the Profile's account is still where it was left is the
/// filesystem's question, and it is asked by whoever prefills with this.
pub(crate) fn platform_default(profiles: &[Profile], role: Role) -> Picked {
    let Some(profile) = HARNESSES
        .iter()
        .find_map(|&harness| account(profiles, harness))
    else {
        return Picked::Nothing;
    };

    let wanted = model(profile.agent_type(), role);

    // The table's model where the Profile lists it, and the first it lists where
    // it does not: nothing makes an account's list carry Fable 5, and a picker
    // left empty over it would be a default that only works for whoever typed
    // the ids this table happens to use.
    let model = profile
        .models
        .iter()
        .find(|listed| *listed == wanted)
        .or_else(|| profile.models.first());

    match model {
        Some(model) => Picked::Under(store::Pairing {
            profile: profile.clone(),
            model: Some(model.clone()),
        }),
        None => Picked::Nothing,
    }
}

/// The one Profile of `harness` a default would run under: the unnamed one,
/// which is the harness's own account where it has one, and otherwise whichever
/// was saved first.
fn account(profiles: &[Profile], harness: AgentType) -> Option<&Profile> {
    profiles
        .iter()
        .filter(|profile| profile.agent_type() == harness)
        .min_by_key(|profile| (profile.name.is_some(), profile.id))
}
