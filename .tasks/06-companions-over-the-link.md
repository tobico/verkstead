# 06. Companions over the link

## What to build

The same leg again for every Companion Repo, which is where a Conversation's
work is more than one repository.

Each Companion's Repo is matched on the far end by the same rule the
Conversation's own is, and its checkout is cut beside the Conversation's the
way it is cut here. What differs between the two kinds is what travels with it.

A **read-write** Companion is a repository a session commits in and leaves
uncommitted work in, so it gets the whole of the previous task: its branch as a
bundle packed against what the far end holds, a binary patch of its tracked
changes, and its untracked unignored files. Its branch is followed where a
rename moved it, which for a Companion on the empty mirroring setting means it
moves with the Conversation's own.

A **read-only** Companion is checked out detached and bound read-only, with
nothing to commit and so nothing uncommitted. It needs its commit and no patch
at all, and it has to arrive detached and bound read-only on the far end —
carrying a patch to one would be carrying changes a session was never able to
make.

The preflight already refuses a press where a Companion's Repo has no match on
the far end, so by the time this runs every Companion has somewhere to land.
What this adds is the carrying.

## Acceptance criteria

- [ ] A Conversation with a read-write Companion arrives with both trees as
      they were: both branches at their commits, both sets of uncommitted
      changes applied.
- [ ] A read-only Companion arrives detached and bound read-only, at the commit
      it was on, with no patch applied to it.
- [ ] A Companion on the mirroring setting whose branch moved with a rename
      arrives under the name it now has.
- [ ] Ignored files in a Companion stay behind, as they do in the
      Conversation's own repository.
