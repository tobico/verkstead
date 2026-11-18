//! **Plugins**: a directory with a `plugin.yaml` at its root, which is how
//! Verkstead is extended and, from the plugins roadmap on, how it is made
//! (ADR-0030).
//!
//! What is here so far is the reading of one — the manifest's grammar, in
//! [`manifest`] — which is what `verkstead plugin check` runs and what every
//! later piece of the loader starts from. The grammar as a plugin's author
//! reads it is `docs/plugins.md`.

pub mod manifest;
