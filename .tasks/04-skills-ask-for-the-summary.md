# 04. Skills ask for the summary

## What to build

One short shared block, worded once and carried identically by every shipped
skill that commits work — next-task, implementing, manual-task and
addressing — telling the session to write the commit's summary as its message
body. The block says:

- **The delivers-the-work rule.** A commit landing code, tests or docs the
  task asked for gets a summary; pure bookkeeping — plan and backlog commits,
  roadmap commits, the finish commit, ADRs recorded along the way — skips it.
  A task's commit still counts as delivering work when the task file's
  deletion rides along with the code.
- **The diagram, first.** Required whenever the diff is more than three
  changed lines, and placed before the prose so the glance comes before the
  reading. The convention is the delta diagram the old gates Topic taught: a
  structure diagram of the changed area, nodes tagged as new, modified or
  removed and coloured to match the diff's added and removed shades, roughly
  ten nodes so it reads on a phone.
- **The prose after it** — what was built and how it hangs together, written
  for the reviewer who reads it before the diff. Trailers go after the
  summary as usual; the workbench strips them from what it shows.

The skills that only do bookkeeping — breaking-down, staging, next-stage —
keep their subject-only commit instructions as they are.

## Acceptance criteria

- [ ] next-task, implementing, manual-task and addressing each carry the
      block, and its wording is one text across the four.
- [ ] The block states the delivers-the-work rule, the three-changed-line
      threshold, the delta-diagram convention and diagram-first ordering.
- [ ] The bookkeeping skills' commit instructions are unchanged, and
      next-task's finish sequence still commits subject-only.
