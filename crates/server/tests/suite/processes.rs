//! Which Process a press names and a view carries, for the suites that name
//! one of the five Verkstead ships.
//!
//! **A Process is the plugin providing it and its id there** (ADR-0030,
//! *Process*), so a test naming *Tinker* names the plugin *Tinker* is — and
//! which plugin that is is the record's own reading of the word it has always
//! held, `store::BUILT_IN_PROCESSES`, rather than a uuid written out again
//! here.

use verkstead_render::ProvidedProcess;
use verkstead_server::store;

/// **Develop**: the ladder, the interview included, and every new Draft's
/// default.
pub(crate) fn develop() -> ProvidedProcess {
    whose("develop")
}

/// **Tinker**: follow-up entered from a Draft.
pub(crate) fn tinker() -> ProvidedProcess {
    whose("tinker")
}

/// **Investigate**: questions about the code answered without changing it.
pub(crate) fn investigate() -> ProvidedProcess {
    whose("investigate")
}

/// **Review**: the wrap-up over the pull request or branch the Target names.
pub(crate) fn review() -> ProvidedProcess {
    whose("review")
}

/// **Fix merge issues**: that wrap-up narrowed to what GitHub refuses a merge
/// for, over the whole stack.
pub(crate) fn fix_merge_issues() -> ProvidedProcess {
    whose("fix-merge-issues")
}

/// The Process the record's word `word` names, as the wire carries one.
fn whose(word: &str) -> ProvidedProcess {
    let process = store::Process::named(word).expect("one of the five Verkstead ships");

    ProvidedProcess {
        plugin: process.plugin,
        id: process.id,
    }
}
