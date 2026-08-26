# 03. Bring the batch sessions under the same rule

## What to build

The batch comment sessions — the wrap-up's other propose-then-fix shape —
carry the same owed-work safety net the review does: a dead batch session's
answered Set is dispatched from the record, and what it owed is computed from
stored findings. Give them the rework task 02 gave the review, so the two
lifecycles read the same way:

- A batch session that ends cleanly settles its batch, and its comments stay
  addressed.
- One that dies is a stop: its unanswered Set is closed, its comments are
  marked unread again — as the existing dead-batch path already does — and
  Resume reads them afresh in a session as new as the first. Nothing is
  dispatched from the record, answered or not.

Accepted batch fixes land in the live session only, which task 01's ending rule
already sees out cleanly.

## Acceptance criteria

- [ ] A batch session that ends cleanly settles the batch without any findings
      record being read.
- [ ] A dead batch session — mid-ask or after the answers — stops the run,
      closes its unanswered Set, and puts its comments back to unread, so
      Resume proposes about them afresh; no fix session is dispatched from the
      record anywhere in responding.
- [ ] The responding tests covering the owed and unattended paths are replaced
      by tests of the new endings.
