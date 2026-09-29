# 08. Transfer back

## What to build

A transfer to a device that already holds a copy of this Conversation, which is
the ordinary case the second time work moves: a Conversation drafted on the
laptop, worked on the Windows machine, and coming home.

The birth key is what says the two are one Conversation. Where the target
already holds a copy under this birth key, the arriving record **replaces that
copy wholesale, under its existing local id** rather than landing as a new
Conversation. So the id the target issued the first time is the id the work
comes back to, which is what makes every link anybody kept work — a bookmark, a
Timeline reference, a URL in a Set answered months ago. Nothing is merged by
hand and nothing is reconciled: what the source has is the live record and what
the target holds is a stale copy, so the stale one goes and the live one takes
its place.

The Worktree at that end is the same question. The target still has the
checkout it cut the first time, on the branch, at whatever commit the work was
at when it left — so the branch is fetched into it and the working changes
applied over it rather than a second Worktree being cut beside the first. The
source's Worktree is left where it is, as it was the first time.

Afterwards the direction is simply reversed: the device the work left keeps its
copy as a tombstone marked transferred, its URL redirects to where the work now
is, and the Merged List draws one row. A third leg lands on the same two ids
again.

## Acceptance criteria

- [ ] Transferring back to a device that holds a copy replaces that copy under
      its existing local id, rather than creating a second Conversation there.
- [ ] Every link to that original id works and shows everything the other device
      added while it held the work — its Timeline, its Sets, its commits.
- [ ] The sidebar holds one row throughout, and after the return it is the
      returning device's; the device the work left redirects to it.
- [ ] A second round trip lands on the same two ids as the first, not on new
      ones, and the Worktree at each end is the one that was already cut.
