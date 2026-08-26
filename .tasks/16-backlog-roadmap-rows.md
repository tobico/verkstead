# 16. Backlog and roadmap rows

## What to build

Give the task list and stage list their natural place on the Timeline. They
have no stamp today — both are read live off the worktree — so the server
stamps a record row when a backlog or roadmap **first lands** on the branch:
the runner already detects that landing to move the conversation on, and the
new row is written at that moment, following the stamped-row-beside-event
transaction pattern the pull request uses (new event kinds, empty bodies —
the row fixes a position, not content).

The Timeline then draws the task list / stage list card at that row, with the
card's content still read live at view-assembly time exactly as the sticky
copy is — one source of card data, two placements. The sticky pinned block
stays as it is. Conversations from before the stamping existed have no row
and simply keep sticky-only cards; do not backfill.

Revise the code documentation that says these events have no place on the
record and nothing opens them, where this task makes it untrue.

## Acceptance criteria

- [ ] A backlog landing on a fresh conversation puts a task list card on the
      record at that moment, ticking live as tasks complete; likewise a
      roadmap and its stage list
- [ ] The sticky block is unchanged, and pre-existing conversations still
      render without a record copy
- [ ] Store, runner and web tests cover the stamped rows
