# 03. Steer into Grilling

## What to build

A new round. The modal grows an optional new brief and a choice about the
digest, and the steer recreates whatever the Conversation is missing before it
starts grilling.

**The brief.** What the human types into the modal lands as the new round's
**Brief Event, frozen at once** — a Brief freezes the moment its round leaves
Draft, and this round has no Draft to leave. It is a second Brief Event beside
the first rather than an edit of it: what the earlier round was built from stays
on the record. A steer without one leaves the Steer Event alone, and the session
starts on the Brief that is already there.

**The digest** is everything the human has already answered — every answered
Question Set of the Conversation, in the order it was asked — which is what a
relaunched grilling already assembles for itself. Here it is **offered as a
choice**, not always sent: a fresh brief is often the point of the steer, and
priming it with the whole of the last interview would be steering into the
argument that has just been left behind.

**The round before is over**, so its wrap-up bookkeeping is forgotten — the same
forgetting a reopened Conversation does — and the round that follows waits on
nothing from the one before.

**The Pairing** shown for this target is the **grilling** one, prefilled from the
Conversation's own and recorded as the Conversation's, the way the implementation
Pairing is for the other targets.

**Recreating what is missing reaches its full extent here.** A Worktree from the
branch where the directory has gone; the **branch itself for a Draft**, which has
never been grilled and so has neither branch nor Worktree — the base commit is
resolved and recorded as a grill start does it. A closed Conversation is a
source like any other: its Worktree was deleted and its branch kept, so the
steer checks the branch out again into a Worktree and carries on.

## Acceptance criteria

- [ ] A steer into Grilling with a brief leaves a new frozen Brief Event plus
      the Steer Event; one without leaves the Steer Event alone.
- [ ] The digest primes the session only where it was asked for, and the session
      is started on the round's own Brief either way.
- [ ] A Draft steered into Grilling gets its branch made, its Worktree checked
      out and its base commit recorded; a closed one gets a Worktree back on the
      branch it kept.
- [ ] The last round's wrap-up bookkeeping is forgotten, and the grilling
      Pairing picked in the modal is recorded as the Conversation's.
