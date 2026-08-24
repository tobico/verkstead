# 05. Red checks fold into the woken session

## What to build

The wrap-up session holds the Worktree for as long as it waits on the review's
answers, so a check that goes red mid-wait cannot get a fix session of its
own. Instead of queueing it behind the wait, the woken session deals with it:
the reviewing skill instructs the session, once the answers arrive, to read
the pull request's current check state and fix whatever is failing alongside
the approved findings, before its push.

A folded check fix spends none of the check's two automatic attempts — that
counter exists to stop unattended loops, and this fix rides work the human
just approved. The checks watcher is otherwise unchanged: ungated addressing
sessions, two attempts, then an Interruption, whenever the Worktree is free
for it.

## Acceptance criteria

- [ ] The reviewing skill has the woken session read the current check state
      and fix failures alongside the approved findings.
- [ ] A folded check fix leaves the two-attempt counter untouched.
- [ ] The checks watcher's own flow is unchanged when no wrap-up session holds
      the Worktree.
