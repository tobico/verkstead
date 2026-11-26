//! What a session's HOME holds that is a fact about this machine rather than
//! about the suite doing the looking.
//!
//! One module rather than a copy per suite, because what it reads is one fact
//! about one machine, and every suite here that lists a session's HOME wants
//! the same answer: two spellings of it would be two accounts of what this
//! machine gives a session, and the one that was written down is the one that
//! goes red on the Annex box and nowhere else.
//!
//! **A module of its own rather than a function inside a suite**, because the
//! suites asking are not all on one platform. `sandbox` is Linux's alone and
//! `sessions` runs on every unix, so an answer kept in the first is one the
//! second cannot see on a Mac — a build failure on that machine and no other,
//! which is what sent this module here.
//!
//! Gated no narrower than its callers, for that same reason, and that is the
//! whole of why it is gated at all: nothing under Windows asks, because
//! nothing is placed in a Windows profile to ask about. A suite that comes to
//! ask from there widens this to match rather than spelling its own answer.

#![cfg(unix)]

use verkstead_server::platform::Platform;
use verkstead_server::sandbox::{machine_path, stack_extension};

/// What a session's HOME holds beyond its account's own, said by the machine
/// rather than written down: `.local`, where this machine has a `gh-stack` on
/// the `PATH` a session is handed.
///
/// The one entry of a HOME listing that is a fact about the host. `gh` looks
/// for an extension inside the home it runs under, so the file is bound at
/// `~/.local/share/gh/extensions/gh-stack/gh-stack` and bwrap makes the
/// parents — see `sandbox::stack_extension`, which is asked here so that a
/// listing reads the same on a machine with one and a machine without.
/// Trailing space included, the probe's listing being separator-terminated.
///
/// Nothing is placed on the platforms that answer `None`, so what this says on
/// a Mac is that a HOME listing there gains nothing — the same sentence as a
/// Linux machine with no extension to give.
pub(crate) fn and_the_stack_extension() -> &'static str {
    match stack_extension(Platform::HERE, &machine_path(Platform::HERE)).is_some() {
        true => ".local ",
        false => "",
    }
}
