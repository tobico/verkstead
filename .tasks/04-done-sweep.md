# 04. Watch Done Conversations' pull requests

## What to build

Keep the mergeable fact fresh after Done, so a pull request that conflicts
while it sits waiting to be merged is noticed without anybody looking.

A sweep on its own pace — every 15 minutes, following the stall sweep's shape:
started at server startup, running for the life of the process — walks every
Done Conversation's recorded pull requests that are not yet known to be merged
or closed, and asks `gh` about each: its mergeability and its state, one call
per PR. The mergeable reading lands in task 01's table; merged-or-closed is
recorded beside it, and a PR once recorded merged or closed is never polled
again — that is the sweep's end condition, learned from the same call that
watches for conflicts. Done only: Wrapping has its own watcher, and Closed is
the human finished — a Closed (and so an Archived) Conversation is never
polled. A `gh` that cannot answer changes nothing and is asked again next
sweep.

Opening a pull request's details pane already asks GitHub about the PR on its
way to listing the checks; extend that ask to fetch mergeability and state too,
and write them down through the same recording — so the pane freshens the fact
the way it already freshens a stale rollup, whatever state the Conversation is
in.

Nothing is dispatched from here and nothing moves: after Done this is watching
only, and what to do about a conflicted Done PR is the human's press (task 06).
No device push and no Notice — the indicator (task 05) is how it catches the
eye.

## Acceptance criteria

- [ ] A Done Conversation's open PR that GitHub reports CONFLICTING has the
      fact recorded within one sweep, with no session dispatched and no state
      moved.
- [ ] A PR recorded merged or closed is never asked about again; Closed and
      Archived Conversations are never asked about at all.
- [ ] Opening the details pane freshens the recorded mergeability and state
      in the same act that freshens the rollup.
