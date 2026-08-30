# 04. The Paths card and pane

## What to build

One new word-named section on the /settings page: **Paths** — the name the
human picked — holding the watched paths and the global sandbox binds
together in one card and one details pane. Per-repo binds are not here; they
are task 05, on each Repo's own pane.

Follow the page's established shape end to end: the word joins the settings
openings list (which is what makes the route exist), the card sits in the
middle pane and summarises, the pane beside it edits, and everything reads
and writes through the one settings query and endpoint from task 03.

On the card: enough to scan — how many watched paths and binds stand, and
whether anything is wrong (no watched path at all, or an entry the server
cannot see).

On the pane, two lists:

- **Watched paths** — settings-owned rows editable (add and remove),
  installer rows read-only and labelled as the installation's. With no
  watched path anywhere, the pane says so and says what it costs: nothing
  can be registered. That is the state a fresh standalone install opens in.
- **Sandbox binds** — the global ones, same row treatment. Each entry widens
  what sessions can write to, so the editor carries a word of caution the
  way the build-cache pane states its boundary — brief, beside the editor,
  not a confirmation step.

Every row reports resolution from the view: an entry the server cannot
currently see says so on its row, in words (which on a nix install is how a
settings-added path says it needs the installer's namespace widened).

Saves follow the page's idiom: rows save on an explicit press, untouched
fields ride along, and the response's read-back view is what the page then
shows.

## Acceptance criteria

- [ ] A Paths card and pane exist at their own settings path, and a path
      naming them survives reload the way the other word-named panes do
- [ ] Watched paths and global binds are added and removed from the pane;
      installer entries draw read-only and labelled
- [ ] An entry the server cannot see is marked on its row; with no watched
      path anywhere the pane and card say so and what it costs
- [ ] Web tests cover the new section the way the existing settings
      sections are covered
