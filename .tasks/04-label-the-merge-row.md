# 04. The merge row says it is a merge

## What to build

A merge commit on the Timeline now carries real counts and a real diff, which
makes it read as an ordinary small commit. It is not one — it is where a base
branch was brought in and its conflicts decided — so its card says so, quietly:
a label beside the sha, in the same register as the Repo label a companion's
commit already carries.

Whether a commit is a merge is read off git when it is described and kept with
the commit, not asked again per page read — the reason the subject and the
counts are kept rather than re-read. The column takes a default, so every commit
recorded before it draws unlabelled, which is the ordinary card.

## Acceptance criteria

- [ ] A merge commit's Timeline card is labelled as a merge; an ordinary
      commit's is not.
- [ ] Commits recorded before the flag existed draw unlabelled rather than
      wrongly labelled.
- [ ] The migration adding the column is safe to run twice.
