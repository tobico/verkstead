# 01. Per-companion commit sweeps

## What to build

A commit landing on a read-write companion's branch reaches the Timeline the way
one on the Conversation's own branch does, and says which repository it came
from.

**A watcher per read-write companion, beside the main one.** The branch sweep is
spawned from one place — inside the relay task a session's launch starts — and
each read-write companion gets a watcher of that same shape, reading its own
repository, its own branch and the base commit its checkout resolved to. All
three are on the companion's record already, from stage 01; a branch left empty
there means *mirroring*, and resolving that is the record's own business rather
than this sweep's. **Read-only companions have nothing to sweep**: their
checkouts are detached and bound read-only, so nothing can land on them.

Every watcher is stopped and awaited when the relay ends, exactly as the one is
now. That last sweep is the point of awaiting rather than only telling them to
stop — a session's final act is usually a commit, and it lands a poll after the
process that made it has gone. One session may now be watching several branches,
so none of them may be forgotten on the way out.

**A commit's identity gains its repository.** The commits table is unique on the
Conversation and the sha today, and the sweep asks which shas are recorded for
the whole Conversation. Both become per-repo: the repo is a column on the row,
the unique index is the Conversation, the repo and the sha, and a sweep asks
what is recorded *for the repository it is sweeping*. Rows already there are the
Conversation's own repository's, so the one-time rewrite that fills them in and
rebuilds the index belongs with the other rewrites the store runs as it opens —
this is what that module is for.

**The label is drawn where a commit is drawn**, on the Timeline card and in the
details pane, and only for a commit that is not the Conversation's own
repository's: an unlabeled card means the work's own repo, and the label earns
its place when repos mix. The label is the Repo's registered name.

**A companion commit's diff is read out of its own repository.** The details
pane takes the repository off the Conversation today, which for a companion's
commit would be the wrong one — it follows the repo the commit was recorded
against, and a commit whose repository can no longer say anything about it is
the *gone* it already is.

## Acceptance criteria

- [ ] A commit made in a read-write companion's worktree appears on the Timeline
      within a sweep, labeled with that repo's name, and the last commit before
      a session ends is caught for every repo being watched.
- [ ] Opening it shows its Commit Summary and its diff, read from the
      companion's repository.
- [ ] The Conversation's own repository's commits draw exactly as they do now,
      unlabeled, and a Conversation with no companions is unchanged throughout.
- [ ] A database written before this opens with its commits attributed to the
      Conversation's own repository, and opening it twice rewrites it once.
