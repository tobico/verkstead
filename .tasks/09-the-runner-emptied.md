# 09. The runner emptied

## What to build

With the three Activities and the investigating configuration running as
components, take the driver code they replaced out of the server: `runner.rs`'s
drivers for the inline, handoff, instruction, submitting, grilling,
investigating and wrap-up paths; `wrapping.rs`'s move-into-Wrapping kept only
as the moment that starts the wrap-up Activity; `checks.rs`, `comments.rs`,
`review.rs`, `merges.rs` reduced to what the host functions call; and
`grillings.rs`, `handoffs.rs` and `investigations.rs` reduced to what the host
functions still need (proposals, the handoff's path, `homeward`). Where these
Activities are called by name stays at `start_grilling` and `take_up` until
stage 11.

What is left of `runner.rs` is the backlog and stage runners, which stage 10
crosses. Comments and module docs that still describe the old drivers are
brought into line.

## Acceptance criteria

- [ ] No driver code for these paths remains outside the host functions and the components
- [ ] The Plugins page lists *Interview*, *One session*, *Wrap-up* and *Follow-up rounds* under Built-in
- [ ] The sessions suite passes at CI's default thread count, and clippy is clean
