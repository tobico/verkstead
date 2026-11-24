//! **Plugins**: a directory with a `plugin.yaml` at its root, which is how
//! Verkstead is extended and, from the plugins roadmap on, how it is made
//! (ADR-0030).
//!
//! What is here so far is the reading of one — the manifest's grammar, in
//! [`manifest`] — and the running of its code, in [`component`]: the two
//! `verkstead plugin check` runs and every later piece of the loader starts
//! from. The Plugins Verkstead ships go through the same two, out of the
//! binary, in [`built_in`]; the ones a human installed, and the Plugins section
//! over them, in [`installed`]. The grammar as a plugin's author reads it is
//! `docs/plugins.md`, and
//! the interface its component is written against is `wit/plugin.wit`.

pub mod built_in;
pub mod component;
pub(crate) mod git;
pub(crate) mod installed;
pub mod manifest;
pub(crate) mod settings;
