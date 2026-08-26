# 15. The PR card twice

## What to build

Pinned cards are to show twice — once in their natural place on the Timeline
and again in their sticky spot. This task establishes the pattern with the
pull request, the one pinned card that already has a stamped record row: stop
folding it out of the Timeline, so the PR card appears both in the sticky
pinned block (as today) and on the record at the moment the pull request
reached it. Both appearances are the same card with the same behaviour —
opening the details pane.

The wire shape will need the PullRequest event admitted to the timeline's
event union (it is currently delivered only through the pinned list); keep
the pinned copy being fed as it is so the sticky block is unchanged.

## Acceptance criteria

- [ ] A conversation with a pull request shows its card in the record at its
      stamped position and in the sticky block
- [ ] Both open the same details pane; selection state reads correctly on
      whichever was tapped
- [ ] Rust and web tests pass, fixtures regenerated where the view changed
