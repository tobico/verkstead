# 05. Clear

## What to build

A **Clear** per language on the **Language support** pane, beside its size and
disk use, that empties every store of that language, sccache's included for
Rust (C/C++ has no store of its own and no Clear).

**Refused while anything runs** (Set 996): the button is disabled while any
session or Conversation Terminal is running, saying how many of each, and the
server refuses a Clear that arrives anyway — the sweep's own gate, from task 04.
A Clear goes through the same machinery as the sweep: every unit, or the store's
contents, renamed aside on the same filesystem and then deleted, launches
waiting on it, read-only directories made writable, symlinks not followed. The
Compile Server's store is cleared with the server stopped or told, so sccache's
own index does not go on describing files that are gone.

The pane's disk use reads zero (or what refilled it) afterwards, and the next
install refills the store.

## Acceptance criteria

- [ ] Clear empties every store of one language and no other language's, and the pane's disk use for it drops accordingly.
- [ ] Clear is disabled on the pane, and refused by the server, while a session or a Conversation Terminal runs, saying how many.
- [ ] After a Clear, a real install in a Sandbox (one of the stage 02–04 proofs) fetches again and refills the store, and Rust compiles again through a cleared sccache.
