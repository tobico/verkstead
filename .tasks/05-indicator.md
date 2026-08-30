# 05. Draw the conflict indicator

## What to build

Show the recorded conflict on the workbench: the pinned pull request card
draws a conflict mark beside the check rollup icon whenever the last look said
CONFLICTING, and the details pane says the same in words.

One recorded fact drawn one way everywhere — the grilling settled that it
draws in any state, Wrapping and Done alike, not Done only. Like the rollup,
it is never guessed at: a PR nothing has asked about draws nothing, MERGEABLE
and UNKNOWN draw nothing, and the mark disappears the moment a fresh reading
says the conflict is gone. The card's read-aloud label carries it, the way the
rollup's word is carried, so a screen reader hears what the icon shows.

The fact travels the existing route: the API types grow the mergeable state
beside the rollup on whatever the card and pane already read, the server's UI
layer serves it off task 01's table, and the existing Nudge on the
Conversation is what tells an open page to re-read — record the reading only
when it changed, so a page is not re-reading a Timeline nothing happened on.

## Acceptance criteria

- [ ] A PR whose last reading is CONFLICTING draws the mark on its card and
      names the conflict on its details pane, in any Conversation state.
- [ ] MERGEABLE, UNKNOWN and never-asked draw nothing, and a reading that
      changes takes the mark away without a reload.
- [ ] The card's read-aloud label says the conflict when the mark is drawn.
