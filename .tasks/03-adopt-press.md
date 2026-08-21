# 03. Adopting — the press that starts the stage

## What to build

The Adopt press, on its happy path: one press turns an adopting Draft into a
stage Conversation that is Implementing, with a planning session running in it.

What it does, in this order, because the order is the point:

1. **Re-read the stage** at the commit the base resolves to. What the page
   showed is a reading of a moment ago, and the repository may have moved.
2. **Git first.** The branch is made at the **stage's own slug** — the brief's
   filename without its number, so `04-wrap-up.md` becomes `wrap-up`, the same
   rule the unattended start uses — off the resolved base commit, and a
   **Worktree** is made with it. Adoption **never stacks**: there is no
   predecessor Conversation to stack on, and stacking on an unmerged
   predecessor is done by the human setting the base commit to its tip.
3. **Then the store hears about it.** The Conversation's branch becomes the
   slug, the stage brief is saved as the **Brief**, and the Conversation moves
   to Implementing with nothing recorded as stacked on.
4. **Then the Timeline gets both records**: the stage brief as its Brief Event,
   and an adoption record saying which stage of which roadmap was adopted and
   where its branch came off. That record is the wording the unattended start
   uses for the same moment, with two adjustments — the human pressed this, so
   *with nobody asked* goes; and adoption never stacks, so only the came-off
   variant applies.
5. **Then the planning session launches**, in the new Worktree, exactly as it
   would for a stage started by a settling predecessor. Nothing about that
   session or what follows it is new: from here the existing pipeline works the
   backlog the plan writes.

Git before the store, always. A row saying work is under way with nothing
checked out is a Conversation nothing can run and nothing can clean up; a
directory on disk the store does not know about is a directory to tidy, which is
the lesser of the two. Every blocking git and filesystem read goes off the
runtime's threads.

Refusals are task 04. This task lands the path that works, and may leave the
refusals as whatever falls out of the store's own checks.

## Acceptance criteria

- [ ] One press leaves the Conversation on the stage's slug branch at the
      resolved base commit, Implementing, with nothing recorded as stacked on.
- [ ] Its Timeline carries the stage brief as its Brief Event and an adoption
      record naming the stage, the roadmap and where the branch came off.
- [ ] A planning session is running in the new Worktree, and the branch and
      Worktree exist before any of it is recorded.
