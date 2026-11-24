//! A built-in plugin that provides nothing, whose wasm component exports
//! nothing: the proof that a component built for `wasm32-wasip2` once is
//! embedded in every platform's binary and loaded at start, ahead of the first
//! built-in that does something (ADR-0030). It is not listed on the Plugins
//! page (Set 1772, Q4).
//!
//! Written against `wit/plugin.wit`'s base world, which asks for nothing, so
//! there is nothing here to bind.
