# 07. Extend the rescue to every session

## What to build

Generalize task 06's rescue from Follow-up to every session Verkstead
launches. The condition and the mechanism are one thing everywhere — a
running session that is idle, with no Set open, and whose work is not done —
and what differs per state is only the **done-indicator**:

- a grilling's artifact — the handoff, the committed backlog or the
  committed roadmap — has not landed;
- a backlog step's task file is still in the Worktree, or the commit
  removing it has not landed;
- an instruction, fix or finish session has not committed;
- a follow-up's latest settled Set carries no end mark (task 06's case,
  now one instance of the general rule).

The mechanism is 06's unchanged: after the grace, the canned line typed into
the session; **two rescues, then a deliberate stop with a Notice** saying
the session went quiet without finishing, in every state alike. Today a hung
step or a grilling that goes quiet without asking sits indefinitely with
nothing saying so; rescued and then stopped, it lands in front of the human
with Resume on offer instead.

Sessions legitimately waiting are never rescued: one sitting on a Blocking
Ask has a Set open, and one actively working is not idle. The rescue
implementation stays in one place, taking the state's done-indicator as its
parameter, so an eighth state later is a new indicator rather than a new
mechanism.

## Acceptance criteria

- [ ] A grilling session that goes idle having asked nothing and landed no
      artifact is rescued, and stopped with a Notice after two rescues; the
      same holds for a step session that went quiet without its commit.
- [ ] A session waiting on a Blocking Ask, or one still printing, is never
      rescued.
- [ ] Follow-up's rescue from task 06 runs through the shared
      implementation, not a copy.
