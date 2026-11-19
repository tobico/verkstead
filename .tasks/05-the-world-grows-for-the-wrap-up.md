# 05. The world grows for the wrap-up

## What to build

Stage 08's `activity` world lends only `launch`, `wait`, `git-facts`, `notice`
and `stop`. The wrap-up needs more, and this task adds it to the WIT, the host
(`lending.rs`), and a fixture component in the suite that exercises each — no
wrap-up logic moves yet. Settled in Set 1838:

- **An event loop rather than blocking on one session** (Q2). Waiting takes
  several sessions and a timeout and answers whichever came first — one of
  them's `waited`, or a tick when the timeout runs out — so one component can
  run four watchers by polling each on its tick, the way the tokio tasks run
  beside each other today. The single-session `wait` keeps working for the
  Activities already written.
- **A `forge` host function**, wrapping `github.rs` for what the watchers ask:
  a pull request's state and mergeability, its checks, its comments and
  review threads, replying, and opening one. Stage 12 puts the Forge interface
  behind it and the component does not change.
- **The settled-at-once path.** Where the Forge lacks a capability a watcher
  needs, the call says so and the watcher is settled at once (ADR-0030,
  *Forge*); with only GitHub behind it this is fed by a test seam, so stage 12
  only has to feed it for real.
- **The wrap-up's records stay the host's** (Q3): host functions to settle or
  unsettle a watcher (`store/wrap_up.rs`, what the viewer draws), count and
  read a check's fix attempts (`check_fix_attempts`), and mark and read a
  comment addressed (keyed by GitHub node id; stage 12 makes it the Forge's).
  They are what the viewer shows, not the Activity's memory, so stage 08's
  *stateless across restarts* holds.
- **No own-Set host function** unless a path crossing in tasks 06–08 really
  calls one (Q5); if one does, add it there.

## Acceptance criteria

- [ ] A fixture component waits on two sessions and a timeout and is answered by whichever happens first, and by a tick when neither does
- [ ] The fixture reads a pull request through the `forge` host function, and a call over a capability the Forge is said to lack comes back settled at once
- [ ] The fixture settles a watcher, counts a fix attempt and marks a comment addressed, and the viewer's wrap-up standings show them
- [ ] Follow-up rounds and the components from tasks 01–04 run unchanged, and the suite is green
