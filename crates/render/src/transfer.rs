//! What a device lacks before a Conversation can be moved onto it, and the one
//! question a device answers about itself for it (ADR-0020, *Transfer*).
//!
//! **A reading of the far end, drawn under the device select in the Transfer
//! dialog.** Nothing here is a press: the preflight is what the dialog shows
//! the moment a device is picked, and what refuses **Go** while anything is in
//! the way — by name, because every one of these findings is something to go
//! and put right somewhere rather than a count to report.
//!
//! **Asked of the far end, with one exception.** Whether one of that device's
//! Repos *is* this repository is settled by [`RepoAcross`](crate::RepoAcross)
//! and the rule over it, which runs on the end that is going to act on the
//! answer — the far end sends its registry and the device holding the work
//! applies it. Everything else only the far end can say, so it answers about
//! itself: which harnesses are on its `PATH`, and whether it is answering at
//! all.
//!
//! **And *unreachable* is never *no match*.** They are two different things to
//! tell a human — no match sends somebody to **Open repo** on a machine that
//! may already have the repository, and a machine that is asleep will answer
//! perfectly well tomorrow — so a device that did not answer is the whole of
//! its own preflight, named, rather than a list of everything it failed to
//! confirm.

use serde::{Deserialize, Serialize};

use crate::AgentType;

#[cfg(feature = "typescript")]
use ts_rs::TS;

/// Which of a Conversation's three roles a Pairing is for.
///
/// Here rather than read off a Profile, because what a missing harness has to be
/// named against is the Pairing that wants it: two roles can want two harnesses,
/// and a sentence that said only *Claude Code is not over there* would leave the
/// human to work out which picker to go and change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum PairingRole {
    Grilling,
    Implementation,
    Review,
}

/// One thing the far end has not got.
///
/// Each names the *somewhere* it is to be put right, because each is a different
/// errand: a repository to open over there, a harness to install over there, or
/// a machine to go and wake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub enum Lacking {
    /// The device did not answer, so it said nothing about itself at all.
    ///
    /// The whole of a preflight where it is in one: a machine that is asleep has
    /// not *failed* any of the questions below, and listing them under it would
    /// be this device inventing findings out of a silence.
    Unreachable,

    /// No Repo on that device is this repository — see
    /// [`RepoAcross`](crate::RepoAcross) for what *is* means.
    Repo {
        /// What this device calls the repository, which is its directory's own
        /// name and the only name there is to point at.
        name: String,

        /// Whether it is one of the Conversation's Companions rather than the
        /// repository the work itself is in. Both are refusals; they are not
        /// the same sentence.
        companion: bool,
    },

    /// The harness a Pairing names is not on that device's `PATH`.
    Harness {
        role: PairingRole,

        /// The Agent Profile's name as it reads here, or nothing for the one
        /// account on its harness that nobody named.
        profile: Option<String>,

        agent_type: AgentType,
    },
}

/// What one device lacks, as the device holding the work found it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS), ts(export_to = "types.ts"))]
pub struct Preflight {
    /// The device it was asked of, by the name the human gave that machine —
    /// which is what every one of these findings has to be said against, rather
    /// than sixteen bytes of hex.
    pub device: String,

    /// Everything in the way, in the order a move would meet it: the machine,
    /// then the repositories, then the harnesses. Empty is a device the work can
    /// go to.
    pub lacks: Vec<Lacking>,
}

/// Whether one harness is on a device's `PATH`, as that device answers for
/// itself.
///
/// **Every harness rather than the ones asked after**, which is what makes this
/// a reading of a machine rather than a question with an answer shaped to it:
/// four rows is the whole list, the device that asked picks out the ones its
/// Pairings name, and nothing has to be sent over before anything can be read
/// back.
///
/// Not a viewer type: no browser draws this, and what a browser is drawn is the
/// [`Preflight`] composed out of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessThere {
    pub agent_type: AgentType,

    /// Whether a session of that type could be launched there at all.
    pub there: bool,
}
