# 02. Read afresh off the top of the chain

## What to build

The carry-on stops reading the roadmap off the **settling stage's own worktree**
and reads it at the **top of its roadmap's chain**: the branch of the last stage
to have joined, and the roadmap's own Conversation's branch where no stage has
joined yet. Read at that branch's commit out of the Repo's git directory — the
index, the declarations on its lines and the stage's brief together — the way the
adoption already reads a roadmap at a base commit, rather than out of any
checkout.

Why: with stages worked side by side each worktree holds a `ROADMAP.md` of its
own, and the settling stage's may have been cut before a dependency was edited or
a stage added. **Declarations are read afresh at every start** so that a hand edit
committed to a running roadmap takes effect, and the top of the chain is the
branch that holds the newest of them.

The roadmap's own Conversation is the one with that roadmap recorded against it
and no stage label — it is the foot of the chain while its pull request is
unmerged, and there is no read for it yet. Where neither the chain nor that
Conversation can be found, the settling Conversation's own branch is the
fallback, which is today's answer and the right one for a roadmap's first stage.

## Acceptance criteria

- [ ] A dependency edited by hand and committed to the top of the chain decides
      what starts next.
- [ ] A settling stage whose branch was cut before that edit starts by the edit
      all the same.
- [ ] A roadmap no stage has joined yet is read off the roadmap's own
      Conversation's branch, and one with neither falls back to the branch that
      settled.
