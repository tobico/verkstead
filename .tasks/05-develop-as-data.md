# 05. Develop as data

## What to build

Develop — the ladder, every new Draft's default and the reading of every
Conversation from before Processes — is written as a bundled plugin with a tick
like the other four, and run by the engine.

- `grilling` — the Interview, offering all three Directions, each mapped to
  `implementing`. `gone` → `stopped`. The Directions the interview offers come
  from the manifest's `directions` rather than
  `Followers::declared`/`Hosting::offering` (stage 09's stand-in), so the
  chooser draws exactly what Develop maps.
- `implementing` — **one state whose Activity is named per Direction** (Set
  1877, Q2, reaffirmed in Set 1879, Q8): `inline` → One session, `task-list` →
  Backlog runner, `roadmap` → Stage runner. The Conversation's stored Direction
  picks which runs, on entry and on every restart, so every `implementing` row
  already stored reads on with **nothing migrated**. Its transitions cover the
  Outcomes of all three Activities. On the card it reads *Implementing*, as
  today. Task 01's grammar carries the per-Direction form of `activity`; if it
  landed without it, add it here, with the check refusing an Outcome of any of
  the three left unmapped.
- `wrapping` — the Wrap-up; a pull request opened on the way in where the
  work is on none (what `to_a_pull_request` does today after `finished` and
  after an inline `done`). `settled` and `merged` → Done; `closed` and
  `given-up` → `stopped`. One session's `refused` → `stopped`. The Stage
  runner's `finished` and `continued` are mapped to whatever they reach today
  — read it off the runner, don't guess.
- `follow_up` — reached only by a steer (brief `follow-up`), returning to
  `wrapping` on either Outcome.
- `investigating` — Develop's own **detour** (brief `question`), returning to
  the state it was steered from (Set 1765, Q28).
- Steer marks: `grilling` takes a `round`; `implementing` takes an
  `instruction` — a steer there runs the instruction and carries on with the
  Activity for the Conversation's Direction, recorded as inline where none is,
  as today.

With Develop read from its manifest, every built-in runs through the engine:
the Process-specific start paths and drivers the five had in code go.

## Acceptance criteria

- [ ] The whole sessions suite passes with all five Processes read from their
      manifests
- [ ] A Conversation stored before this stage in `implementing` under each
      Direction, or in `grilling`, `wrapping`, `follow_up` or `investigating`,
      reads its state and runs on with nothing rewritten
- [ ] A steer into Develop's `investigating` returns to the state it left
- [ ] Unticking Develop takes it off the picker, and a new Draft defaults to the
      first Process still offered
