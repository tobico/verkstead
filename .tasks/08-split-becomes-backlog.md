# 08. A split pick becomes a backlog

## What to build

The escape hatch, end to end. The reviewing skill offers a Split Option on a
finding only where it judges the work too big to fix in one sitting —
ordinary Sets carry no Split Option at all. The human controls the mix per
finding: fix here, split out, or leave.

On the answers, the woken session fixes and pushes the fix-here picks as
usual, then writes a `.tasks/` backlog for the split picks — `TODO.md` plus
one task file per split finding, each carrying what the review wrote for
whoever fixes it and the human's words beside the pick — commits it, and
ends.

The server, seeing the wrap-up session end with a backlog on the branch,
moves the Conversation from Wrapping back to Implementing — a new lifecycle
move — and runs the backlog as it runs any other: a session per task, then
the finish, which re-enters Wrapping through the second-wrap plumbing and a
fresh review. A Response that accepted no splits changes nothing about the
ordinary clean end.

## Acceptance criteria

- [ ] A mixed pick works end to end: fix-here findings pushed by the wrap-up
      session, split findings run as backlog tasks, and the re-wrap reviews
      the branch fresh.
- [ ] A Set where the session judges no split warranted offers no Split
      Option, and a split accepted with zero fix-here picks also works.
- [ ] The no-dropped-fixes rule reads a split pick as expecting the backlog
      rather than a fix, and an ended session that landed neither raises the
      Interruption.
