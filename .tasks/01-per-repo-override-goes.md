# 01. The per-repo conflict override goes end to end

## What to build

A Repo could say Merge or Rebase for itself, over the top of the global
Conflict resolution setting, from a section on its own settings pane. The human
decided that an override the settings page no longer shows would be a Repo that
rebases with no way to see why, so the override goes rather than hides: the
picker on the Repo pane, the endpoint that set it, the field on the Repo view
that carried it, the store's read and write of it, and the lookup a conflicted
pull request made before falling back to the global setting. Resolving a
conflict reads the settings file and nothing else.

The override lives in a table of its own, created on open because the store has
no migration machinery. Drop that table on open, beside the creates, so an
install that had saved an override comes up clean rather than carrying a table
nothing reads.

The strings and the rebase warning the Repo pane shared with the Conflicts card
stay where the card owns them for now; task 03 moves the card into the Git
pane, and this task only stops the Repo pane importing them.

Tests follow the code out: the web tests for the Repo pane's picker, the server
and store tests for the endpoint and the table, and the checks test that
exercised the override path. The design doc's settings block says a Repo's pane
carries its resolution; it no longer does.

## Acceptance criteria

- [ ] A Repo that had an override saved rebases or merges by the global setting on its next conflicted pull request.
- [ ] Nothing in the web client or the server refers to a per-repo resolution, and the store neither creates nor reads the table.
- [ ] The design doc's settings block no longer says a Repo's pane holds a resolution.
