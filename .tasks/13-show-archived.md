# 13. Show archived, and Unarchive

## What to build

The way back from task 12's archiving:

- **A "Show archived conversations" toggle** in the conversations-list
  pane's ⋯ menu. On, archived conversations appear in the list (in their
  ordinary place, dimmed as ended cards are); off, they are hidden. The
  toggle's state persists server-side the way the hand-made list order does,
  so it survives reloads and devices.
- **Unarchive**: an archived conversation's ⋯ actions menu offers Unarchive
  in place of Archive, returning it to the list for good.

## Acceptance criteria

- [ ] The toggle reveals and hides archived conversations, and its state
      survives a reload
- [ ] Unarchiving returns the conversation to the ordinary list with the
      toggle off
- [ ] Route and web tests cover the toggle's persistence and the unarchive
      round trip
