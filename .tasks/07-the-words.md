# 07. The words

## What to build

The two documents that still say a Stage's branch stacks on its unmerged
predecessor, brought up to what this stage built.

**`docs/design/verkstead.md`'s *Stages always stack* bullet.** Its *the next
stage's branch stacks on the unmerged predecessor* is the sentence the chain
replaces. It is **refined rather than rewritten**, in the document's own
*(refined DATE, building STAGE)* form — that bullet already carries two such
refinements and the form is what lets somebody read back how the decision moved.
What the refinement says: a Stage is cut from the highest settled Stage in the
chain, and **the join is at the finish** — it waits until every Stage already in
the chain has settled, rebases onto the top, and only then pushes and opens a
pull request, so no pull request anybody has started reading is ever rewritten.

**`CONTEXT.md`'s Stage entry.** *Its branch stacks on the unmerged predecessor
where the target repository records how, and comes off the default branch where
it does not* is the same claim one level down, and it is now two facts rather
than one: where the branch is **cut from**, and what it ends up **stacked on**.
Revise it in the glossary's own voice, and keep the *Avoid* line's job of saying
what not to call it.

**And a Chain entry of its own**, which the glossary has no term for. A chain is
one per roadmap, in the order its Stages finish, its bottom the roadmap's own
branch while that is unmerged and the default branch when nothing unmerged is
left to stand on. A Stage joins it at its finish, one at a time, after every
Stage already in it has settled; a Stage that has joined is never rebased by a
later one, and what moves it afterwards is the sync any stacked branch gets. Say
what the joins being in single file costs, and cross-reference ADR-0021 the way
the other entries cross-reference their ADRs.

No rewriting of the surrounding entries, and no restating the ADR: both
documents reference it rather than carrying it.

## Acceptance criteria

- [ ] The design document's *Stages always stack* bullet is refined in its own
      *(refined DATE, building STAGE)* form rather than rewritten, and says the
      join is at the finish.
- [ ] `CONTEXT.md`'s Stage entry no longer says the branch stacks on the
      predecessor as the whole of the story, and tells the base it was cut from
      apart from what it ends up stacked on.
- [ ] `CONTEXT.md` gains a Chain entry, in the form the other entries take, with
      its own *Avoid* line and a reference to ADR-0021.
- [ ] Nothing else in either document is reworded, reordered or renumbered.
