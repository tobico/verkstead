# 09. `Lifecycle` goes

## What to build

A state is a word read against a manifest. The `Lifecycle` enum in the store,
its render mirror and its ts-rs export go, along with every match on them —
about 1,185 references across the tree at planning time, some 800 of them in
the store and server suites, so expect most of the work to be tests.

- The store's per-state moves (`follow_up_over`, the implementing and wrapping
  moves, and the rest of the ~80 functions that each know one state) become
  moves to a word, with what a move must check — the state it moves from — read
  off the manifest by the caller. Draft, Done and Closed (and the old
  `aborted` spelling of Closed) stay Verkstead's, named in the store.
- `resume.rs`, `drivers.rs`, `stalls.rs`, `steering.rs`, `boundaries.rs`,
  `reports.rs`, `transfers.rs` and the rest read the manifest instead of
  matching on a state.
- A Transfer carries a Conversation of a third-party Process: the receiving
  device reads its state against the same plugin, and one lacking the plugin
  refuses the move saying so, as a lacking Agent is refused.

## Acceptance criteria

- [ ] No `Lifecycle` anywhere in the tree, the generated types included
- [ ] A Transfer carries a Conversation of a fixture third-party Process and it
      runs on over there; to a device without the plugin it is refused naming it
- [ ] The whole Rust suite and the web suite pass
