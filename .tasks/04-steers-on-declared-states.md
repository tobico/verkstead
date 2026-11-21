# 04. Steers land on declared states

## What to build

Every Steer target is a declared state (Set 1765, Q28). The Steer form's rows,
and what the server does with a steer, come from the steer marks of the
Conversation's Process instead of the fixed `SteerTarget` set.

- A state's steer mark names one of Verkstead's **brief kinds** (Set 1877, Q5),
  each of which Verkstead knows how to draw and hand on:
  - `round` — a new round's Brief, and how much of the last interview to carry;
  - `instruction` — what the human wants done (or nothing, carrying on, where
    the branch has something standing);
  - `follow-up` — what to take up about work already pushed;
  - `question` — the question an investigation answers;
  - `none` — the steer takes no brief.
  The brief reaches the state's Activity as a start fact / option.
- A **detour** state returns to the state it was steered from when its
  Activity ends, whatever the Outcome: the steer records its source state, as
  ADR-0020 decided, and the detour reads it back. Steered in from a state
  nothing returns to (Done, a Draft), it returns to Done.
- Close stays on every Steer form; it is Verkstead's, not a state.
- The form's rows are drawn from data the server sends — `Steer.tsx`'s
  `TARGETS` and the form's per-target shapes are drawn from the steer marks and
  brief kinds. The roles a target offers come from the target state's Activity
  until task 06 makes roles the manifest's.
- The four Processes from task 03 gain their marks (Tinker's `follow_up` takes
  a `follow-up`, and so on), so the existing steer tests for them pass through
  the new path. Develop's marks come with Develop in task 05; until then it
  keeps its present steer path.

## Acceptance criteria

- [ ] A fixture Process with no steerable state draws a Steer form of Close
      alone, and the server refuses a steer to an unmarked state naming it
- [ ] A steer into a fixture detour state returns to the state it was steered
      from when the detour's Activity ends
- [ ] Each brief kind draws its own fields on the form and reaches the
      Activity; the steer tests for Tinker, Investigate and Review pass
