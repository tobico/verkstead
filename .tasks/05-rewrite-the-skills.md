# 05. Rewrite the reviewing and responding skills

## What to build

The bundled reviewing and responding skills still teach the old shape: a
finding locked to fix-or-leave, a `review` block naming the fix and split
Options, a split offered rarely. Rewrite them for the settled design:

- **Each credible way to fix a finding is its own Option**, worded freely —
  "Fix it — reset the counter as the window rolls" beside "Fix it — collapse
  the two clocks instead" — with the recommended way starred. A finding with
  one sensible fix keeps one fix Option; alternatives are offered whenever more
  than one credible way exists, never invented.
- **Leave it is always offered**, so declining stays possible on every finding.
- **The spin-off is an ordinary Option**, offered when a fix is genuinely too
  big for the sitting: picked, the session writes the fresh `.tasks/` backlog
  it already knows how to write, commits it and builds none of it — the server
  reads the backlog off the branch from there.
- **No `review` block anywhere.** The Set a review or batch session sends is a
  plain Question Set, and the session acts on the answers directly: any picked
  fix way is fixed here, that way, with whatever the human wrote beside it.

Sweep the skills and the `verkstead guide` text for every reference to the
findings block, and update project documentation that describes the old split
marker where it still does.

## Acceptance criteria

- [ ] Neither skill instructs writing a `review` block, and `verkstead guide`
      carries no reference to the findings grammar.
- [ ] The reviewing skill has the agent offer each credible way as its own
      Option with a starred recommendation, leave-it on every finding, and the
      spin-off where it makes sense — ending the spin-off path with the fresh
      backlog committed and nothing built in-session.
- [ ] The responding skill matches, for what a batch of comments proposes.
