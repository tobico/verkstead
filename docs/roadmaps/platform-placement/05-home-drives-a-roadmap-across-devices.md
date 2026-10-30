# 05. Home drives a roadmap across devices

## Goal

A stage of a roadmap can be worked on another device without the roadmap
losing sight of it. The device holding the roadmap's Conversation keeps a
tombstone for each stage moved away; the live copy tells home every lifecycle
move — Wrapping, Done, Closed, a stop — and home writes it onto the tombstone,
so standings, places and the chain are read where they always were. The far
device's scheduler ignores a stage whose roadmap is not its own. A finish on
the far device asks home over the link whether it may join and what the
chain's top is, and waits, said on the Timeline, while home is unreachable.
Demonstrable: a stage transferred by hand to a second device finishes there,
and the first device starts the stage that stood on it.

## Decisions in force

- **Home drives; the stage reports back**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Home drives a
  roadmap across devices*). Replicated stage records were turned down as heavy
  and as two schedulers racing; stages that never move were turned down.
- **The tombstone follows the live copy's state.** What reads the record —
  `stage_standings`, the places, the chain — changes as little as possible:
  the row it reads is kept true instead.
- **The far scheduler ignores foreign roadmaps**: a stage's roadmap row that
  crossed with the slice does not make the far device a driver of that
  roadmap.
- **The join asks home**: whether every stage already in the chain has
  settled, and what branch is at its top. Home unreachable is a wait, said,
  not a refusal.
- **The work stays where it is** — the finish and the wrap-up run on the far
  device, which is why the join has to be askable from there.
- **True today of a hand-pressed Transfer**, which is why this stands on
  nothing here.

## Proposed tasks (provisional)

1. **The report** — a lifecycle move on a Conversation that is a copy tells its
   birth device over the link; home writes the state onto its tombstone.
   - A stage closed on the far device reads abandoned at home.
2. **The far scheduler stands down** — a settle on a device that does not hold
   the roadmap's Conversation starts nothing there.
3. **The join over the link** — the finish's wait-to-join and the chain's top
   asked of home; unreachable waits and says so.
   - A stage that joins on the far device opens its pull request against the
     branch home named.
4. **Coming home** — a stage moved back lands on its tombstone as a transfer
   back does today, the standings unchanged by the round trip.

## Re-verify at start

- The standings are still `stage_standings` in
  `crates/store/src/conversations.rs`, joining `stage_roadmaps` to
  `conversations` with no account of a tombstone; the tombstone is still the
  `transferred` table in `crates/store/src/transfers.rs`.
- The scheduler is still `startable` in `crates/server/src/stages.rs`, run from
  `continuing::settle` in `crates/server/src/continuing.rs`; the join queue is
  still `stage_joinings`.
- The slice still carries `stage_roadmaps`, `stage_branches` and
  `stage_joinings` — see `crates/store/src/slices.rs`.
- The link is still the peer routes under `crates/server/src/peer/` and the
  relay in `crates/server/src/relaying.rs`; the arrival's word to the far end
  is in `crates/server/src/transfers.rs`, and the reverse direction has no
  route yet.
