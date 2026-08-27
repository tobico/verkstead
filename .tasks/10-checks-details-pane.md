# 10. Checks in the details pane

## What to build

The pull request details pane gains a checks section: every check on the pull
request, each with its status icon and its name linking to the associated
build or run on GitHub, using the link GitHub gives (empty where it gave none —
those render as plain names).

The details endpoint already shells out to GitHub at request time for commits
and comments; it fetches the checks in the same breath, returns them with the
rest, and refreshes the stored rollup from task 09 — so opening the pane is
also what freshens a stale card icon on a conversation nothing watches.

The section sits beside Commits and Comments with the same empty and error
manners the pane already has.

## Acceptance criteria

- [ ] The pane lists every check with the GitHub-style status icon and a link
      to its build where GitHub gave one
- [ ] A pull request with no checks says so quietly, matching the pane's other
      empty states
- [ ] Opening the pane updates the stored rollup, so the card icon agrees with
      what the pane just showed
