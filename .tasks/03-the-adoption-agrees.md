# 03. The adoption agrees

## What to build

The **other** reading of a roadmap reads the same record. There are two: the
carry-on reads a Worktree as it stands, which is task 02; the adoption reads a
roadmap at a **commit** in a registered Repo, with no Worktree anywhere in it,
and answers three places off one reading — the abandoned-roadmap notice, the
adopting Conversation's page, and the press itself, with only the press saying
which of its clauses refused.

All three have to agree with the carry-on about what is done, by the same rule:
the record where it has a row for the stage, the boxes where it has none.

**What changes, visibly.** A roadmap whose stage 01 has settled on a branch
nobody has merged is unticked at the default branch's tip, and the tick is on
that unmerged branch. Today the adoption reaches stage 01, finds its branch in
the Repo and refuses — *its branch is taken* — so the roadmap that most needs
carrying on offers nothing. With the record it offers **stage 02**.

**What stays.** A stage the record says is in flight is refused as in flight,
by the record rather than by the annotation naming a branch that still exists —
and that annotation reading stays as the fallback for a roadmap the record knows
nothing about. A stage the record says was abandoned did not settle, and the
branch it left behind is what still refuses it, exactly as today: reopening
abandoned work is not this stage's business.

**One reading, handed the record.** The notice walks every roadmap of every
registered Repo — at the default branch's tip and at every local branch the
default has not swallowed — so what the reading needs is the Repo's stage rows
read once, not a lookup per roadmap inside it. Keep the reading itself a
reading: nothing in it asks the database.

## Acceptance criteria

- [ ] A roadmap whose stage 01 settled on an unmerged branch offers stage 02 at
      the default branch's tip, and the notice, the page and the press all name
      the same stage.
- [ ] A stage the record says is in flight is refused as in flight, and the
      press says so by name.
- [ ] A roadmap the record knows nothing about is adopted exactly as it is
      today, off its boxes, its annotation and its branches.
- [ ] A stage whose Conversation was closed before it wrapped up is still
      refused by the branch it left behind.
