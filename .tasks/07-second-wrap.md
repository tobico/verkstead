# 07. A Conversation can wrap up twice

## What to build

The plumbing a split-out backlog lands on: a Conversation that leaves Wrapping
for Implementing and finishes its backlog wraps up again, cleanly.

Two rules change. Leaving Wrapping resets the Review's settle — *settled once
and stays settled* holds within one wrap, not across re-entry — so a fresh
review runs over what the backlog built. And the ending that moves a
Conversation into Wrapping accepts one whose pull request is already recorded,
reusing the record rather than inserting a second pull request row and Event;
the lifecycle moves already tell the re-entry's story on the Timeline. Today
that path refuses any state but Implementing or Grilling and always inserts
fresh, which would duplicate the record.

The wrap-up's watchers start on re-entry exactly as on the first wrap, and the
checks and comments settle from GitHub's current answers as they always do.

## Acceptance criteria

- [ ] A finish on a Conversation whose pull request is already recorded moves
      it to Wrapping with exactly one pull request row and Event.
- [ ] The Review is unsettled on re-entry, so a fresh review session runs over
      the branch.
- [ ] Store and server tests cover the second wrap.
