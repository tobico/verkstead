# 03. Setup on the brief card

## What to build

A Conversation's setup and kick-off happen in the timeline, not the details
pane. Everything editable in the root details pane today — the branch-rename
field, the base-commit field, the two pairing pickers and the readiness
verdict — moves onto the brief card, **below the brief text**: the brief is
the headline, the setup follows it.

Once grilling starts, the setup section **disappears entirely** and the card
is the brief alone. Nothing collapses into a summary and nothing shows
disabled — this was settled against both alternatives. Branch and base
already freeze server-side at that point, and the pairings lock there too
(task 02), so nothing removed was still actionable.

The three read-only facts the pane also shows — repo name, worktree path,
state — appear nowhere afterwards. They do not re-home to the card or the
header; they are simply dropped (settled explicitly: the timeline already
tells the story, and the chooser declined every re-homing option).

The root details pane becomes bare paper: a details pane exists only for a
selected timeline item with details to show. On layouts where the panes page
between each other, the forward-to-details control hides while nothing is
selected, so there is no way to page into an empty pane.

## Acceptance criteria

- [ ] While drafting, the brief card carries branch, base, both pairing
      pickers and readiness beneath the text; after grilling starts, only
      the brief remains
- [ ] Repo name, worktree path and conversation state are shown nowhere
- [ ] With nothing selected the details pane renders empty, and narrower
      layouts hide the forward-to-details control until a selection exists
