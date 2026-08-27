# 08. The vocabulary

## What to build

Bring `CONTEXT.md` up to date with what the feature added, in the glossary's
own voice and shape — a term, what it is, what it is not, and an _Avoid_
line each:

- **Follow-up**: the state, its one way in (a steer, from Done or Wrapping,
  where the record holds a pull request), the required brief on the Steer
  Event, the rounds of ordinary Sets, and how it ends — the Nothing-else
  mark plus an idle session, landing back in the wrap-up.
- **Waiting on checks**: the derived condition of Wrapping, its Notice, and
  that it is a condition rather than a state — beside *blocked on you*,
  which is its precedent.
- **Nothing else**: the control, drawn only on a Follow-up's Sets, riding
  the Response and invisible to the agent.
- **Rescue**: the condition, the canned line, the two-attempt bound and the
  stop it ends in, and the per-state done-indicators.
- The **Steer** entry updated to five targets with Follow-up's payload
  beside Grilling's and Implementing's, and the **Conversation** entry's
  ladder updated with how Follow-up sits beside it.

The entries describe the built behaviour, so this lands last, checked
against what the tasks actually built rather than what the plan said.

## Acceptance criteria

- [ ] CONTEXT.md carries the four new entries and the updated Steer and
      Conversation entries, consistent with the shipped behaviour.
- [ ] No entry contradicts another — in particular Postscript, which stays
      true: the agent still never asks an anything-else Question, because
      the ending is the control's, not a Question's.
