# 04. Following the disk

## Goal

The tree and the editors follow the Worktree without being asked. An agent
writing a file, a build filling a folder, a terminal tab checking out a branch:
each reaches the page as a `files` Nudge from a watcher on the server, and the
page re-reads what it has expanded and what it has open. A clean editor takes
the new text silently; a dirty one shows the Reload / Keep mine bar the moment
the disk moves rather than at the next save. Files and folders carry git
status marks — modified, untracked — read again on every `files` and `commit`
Nudge.

## Decisions in force

- **[ADR-0019](../../adr/0019-the-code-pane.md), *Following the disk*,
  *Versioned reads*, and *The tree*** — the watcher, its on-demand life, the
  ignore-aware walk, the payload-less Nudge, the reload rules and the marks.
- **A `Files { conversation }` kind joins the Nudge enum** in the schema crate,
  announced by the watcher and mapped on the viewer to the files queries — two
  edits, the way `docs/development.md` says a kind is added. It carries no
  payload (ADR-0009): the page re-reads the folders it has expanded, the open
  files' versions and the status, and a re-read is cheap because a folder is
  one listing and a version is one hash.
- **The watcher runs while a Code pane is attached and stops when none is.**
  How the pane says it is attached is provisional — a socket the pane holds
  open, the way a terminal tab holds its attach, is the leading shape, and a
  request that names the Conversation and is renewed while the pane is open is
  the other. It watches the Worktrees' non-ignored directories, walked with the
  same ignore rules the tree hides by, and adds directories as they appear;
  events are debounced to one Nudge per burst, because a build writes
  thousands of files and the page wants to look once.
- **The version is what tells a real change from a touch.** A Nudge makes the
  page re-read the version of each open file; where it matches, nothing
  happens. Where it differs, a clean buffer takes the new text and keeps its
  cursor as near as it can, and a dirty one shows the bar stage 01 built.
- **Git marks come from one status read per root**, porcelain and
  NUL-separated, folded up so a folder carries the strongest mark of anything
  under it, and drawn in the colours the Diff pane already uses for added and
  changed. Read again on `files` and on `commit`, since a commit clears marks
  without touching the files.
- **The commit's own writes are the Worktree moving too.** `.git/index` and
  `HEAD` are outside the tree but inside what the marks depend on, so the
  watcher watches those two files of each root's git directory and nothing
  else under `.git`.

## Proposed tasks (provisional)

1. **The kind and the watcher** — the `Files` variant, the watcher started
   and stopped with the pane's attachment, ignore-aware, debounced, announcing
   through the nudge stream. AC: a file written by a terminal tab produces one
   Nudge; a `cargo build` produces a handful rather than thousands; closing the
   last Code pane on the Conversation stops the watcher within a moment; a
   worktree with a huge ignored directory starts watching in under a second.
2. **The tree follows** — expanded folders re-read on the Nudge, a folder that
   vanished collapses, a new folder appears in place. AC: `mkdir` in a terminal
   shows the folder; `rm -r` takes it away; collapsed folders cost no read.
3. **Editors follow** — versions re-read on the Nudge; clean buffers reload
   silently; dirty ones show the bar at once. AC: an agent's edit to a clean
   open file appears without a keypress; the same edit to a dirty file shows
   the bar with the human's text intact; Reload takes the disk's text.
4. **Git marks** — the status endpoint per root, marks on files and folders,
   re-read on both Nudge kinds. AC: a save marks the file and its folders; a
   commit in a terminal clears them; an untracked file is marked as such.

## Re-verify at start

- Which stages have landed: the bar this makes earlier is stage 01's; the tabs
  it reloads may be in any group and shared between two views after stage 02.
- The nudge table in `web/src/nudge.ts` is still the one place a kind is
  mapped to queries.
- The workspace still has no filesystem-watcher crate; `notify` is the likely
  one, and its recommended watcher on each platform, and the inotify limit on
  the machines this runs on, are worth reading before the walk is designed.
- The Diff pane's colours for added and changed lines are still the tokens to
  draw the marks in.
