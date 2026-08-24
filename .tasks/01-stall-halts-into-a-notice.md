# 01. A stall halts into a Notice

## What to build

When the stall sweep finds a Conversation in a driven state with nothing
driving it, it records a **halt** instead of raising an Interruption. A halt
is durable state on the Conversation — when it halted, whether it was
*deliberate* (Verkstead pulled the brake, or the human asked) or
*circumstance* (a restart or crash took the driver away), and which Timeline
Event explains it — cleared when driving starts again. The explanation is an
ordinary **Notice**: markdown saying what stopped ("implementing the work"),
why ("nothing is driving it…"), and the evidence an Interruption used to
carry — the worktree's git status and the tail of the last agent output.

The *blocked on you* badge points at that Notice, exactly as it points at an
open Interruption today. The sweep writes one halt and one Notice per stall,
not one per pass: a Conversation already halted is not news. The sweep's
"is there already an open Interruption" condition becomes "is it already
halted".

Interruptions still exist after this task — every other raising site is
untouched until task 02, and the old card, sheet and settle endpoint keep
working for events already on Timelines.

## Acceptance criteria

- [ ] A stalled Conversation gains one Notice with the stop, the reason and
      the evidence, a halt the store round-trips, and the badge — and gains
      nothing further on later sweeps.
- [ ] The stall sweep raises no Interruption anywhere, and the sweep's tests
      cover the halt instead.
- [ ] Clearing the halt (the store call task 04 will press) drops the badge.
