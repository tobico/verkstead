# 01. Add a companion, and take it away

## What to build

The first end of companion repos: a Conversation can be given another registered
Repo to work alongside, and can have it taken away again, for as long as it is
still drafting.

**A companions relation per Conversation**, carrying everything a companion ever
holds — which Repo, its mode, the base ref it comes off, and its branch name.
All four columns arrive now even though only the first two are set in this task:
the task after this one makes the rest editable, and a relation reshaped between
two tasks is a migration nobody needed. It hangs off the Conversation as a side
table of its own, the way the worktree and the direction and the adoption
already do — `conversations` is STRICT and left alone, and there is no migration
machinery here for a new table to need. **The `worktrees` table's
one-row-per-Conversation shape is not touched**; companions get room of their
own when task 03 needs it.

**Adding defaults to the least the human has to say**: read-only, no base ref
(which means that repo's own default branch, the same rule the main base picker
calls the rule), and an empty branch name (which means mirroring — task 02's
business).

**Two things are refused by name.** The Conversation's own Repo is not a
companion of itself, and a Repo already added is not added twice. Both come back
as named refusals the card can say out loud, alongside the ones every drafting
endpoint has — no such Conversation, no such Repo, and not drafting.

**The `Menu` component learns to nest.** It has one level today, and the human's
explicit call was nesting over a flat list of every repository. A nested level is
a row that opens another level rather than doing something, with a way back out
of it, sharing the one card, the one backdrop, the one Escape handler and the
one way the focus is given back — the whole reason that component exists is that
those were written three times and drifted. The other shape in that file, the
right-click menu, gets the nesting for free by sharing the drop.

**The ⋯ beside the branch row** is where it hangs — the first of these drawn
inside a card body rather than at the head of a pane, so the trigger the pane
heads share is not automatically the right paint here. It holds one row, *Add
companion repo*, and that row opens a nested level listing the registered Repos.

**A row per companion under the branch row**, naming the Repo, with a × that
takes it away. In this task the row is the name and the ×; the pickers and the
switch arrive in task 02.

**The rows go when the card freezes**, with the branch and base fields they sit
under. The card already draws that from the Conversation having a worktree, and
a row whose save comes back refused is worse than no row.

## Acceptance criteria

- [ ] The ⋯ beside the branch row opens *Add companion repo*, which lists the
      registered Repos in a nested level of the same menu — one card, one
      backdrop, Escape out of either level, and the focus given back to the ⋯.
- [ ] Picking a Repo draws a companion row under the branch row; × takes it away.
- [ ] The Conversation's own Repo and one already added are each refused by
      name rather than added, and every add and remove is refused once the
      Conversation is no longer drafting.
- [ ] The companion rows go with the branch and base fields when the card
      freezes.
