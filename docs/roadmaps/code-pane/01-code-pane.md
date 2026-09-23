# 01. The Code pane

## Goal

The terminal icon beside Share is the `code` icon, and what it opens is
**Code**: a details pane at `/code` with a file tree down its side over every
Worktree the Conversation has, and one group of tabs beside it holding files
and terminals alike in a tab bar styled after VS Code's. A file pressed in the
tree opens in Monaco; Ctrl+S writes it back, and a write over a file the agent
has changed since is refused with a bar offering Reload or Keep mine. Live
shells come back as tabs; a New terminal button opens another; × on a tab
closes it, asking first where the file is dirty or the shell is busy. A
maximise toggle gives the pane the window. `/terminal` redirects. The Terminal
pane is gone.

## Decisions in force

- **Everything in [ADR-0019](../../adr/0019-the-code-pane.md).** The sections
  that bear on this stage: *One pane, replacing the Terminal*; *The server reads
  and writes the Worktree, outside the Sandbox*; *Versioned reads, and a stale
  write is refused*; *Tabs and groups* (the × rule, the busy rule and the empty
  pane — the groups themselves are stage 02); *Monaco, whole*; *What Code is
  not*.
- **The files API is conversation-scoped under `/api/ui/`**, beside the
  terminals' routes, and answers refusals in the body the way registering a Repo
  does: a path outside a root, a path under `.git`, a root that is read-only, a
  file that is binary or too large, and a version that has moved are each a
  different sentence for the human, none of them a status code. Roots are the
  Conversation's own Worktree and each companion's, in the order the
  Conversation carries them, each saying whether it is writable; the module
  that composes a Set's Diff already lists the writable ones and is the pattern
  to follow.
- **A folder listing is one folder**, read when it is expanded, never a walk:
  the path field's directory browse is the shape. Git-ignored paths and `.git`
  are left out, which git is asked about rather than reimplemented.
- **A read carries a version** — a hash of the bytes — **and a write names the
  version it is over.** The refused-stale answer is what draws the Reload /
  Keep mine bar in this stage; stage 04 makes the same bar appear the moment the
  disk moves rather than at the next save. A read says which kind of thing it
  read: text, an image, a binary it will not send, or a file over the size cap.
- **The busy flag is the server's.** The terminals list and the close both say
  whether a terminal's foreground process is something other than its shell —
  `tcgetpgrp` on the pty the server holds against the shell's own process
  group, on the platforms with a pty. Where the platform cannot answer — the
  ConPTY on Windows — the flag reads busy and every close confirms.
- **Code's state is held above the details pane**, so swapping the pane for an
  Event and back finds the tabs, the buffers and the dirty text where they
  were. The device's storage that survives a reload is stage 02's; this stage
  survives the swap and warns on leaving the page with dirty text, the
  browser's own way.
- **Monaco is a chunk the page loads when Code first opens**, with its workers
  beside it under the hashed assets path, and it follows the workbench's theme.
  Its settings are stage 03's; here it is VS Code's defaults.
- **The maximise toggle is remembered per device**, beside the pane widths in
  `localStorage`, off on the first open, and does nothing below the breakpoint
  where the details pane already has the window.
- **Wire types are generated** from the schema crate by `cargo test` into
  `web/src/api/types.ts`, and the vitest suite is fed golden fixtures the real
  endpoints write. New endpoints regenerate both; nothing is hand-written on
  the viewer side.
- **Nothing here is a record**, and every place the workbench says so of a
  terminal says it of Code.

## Proposed tasks (provisional)

1. **The pane and the redirect** — Code at `/code` with the `code` icon where
   the terminal icon was; `/terminal` redirects; the Terminal pane's terminals,
   tabs and refusal states move into one group under a tab bar restyled after
   VS Code's, with a kind icon and a × on every tab and the empty state with its
   hint and New terminal button; nothing opens on load. AC: every terminal test
   passes against the new pane; the pane opens empty with live shells as tabs;
   pressing × on a terminal ends its shell; the old path lands on the new one.
2. **Busy shells** — the server reports the flag on the list and on close; a ×
   on a busy tab confirms, and on a platform that cannot tell always confirms.
   AC: a `sleep` in a terminal makes its × ask; an idle prompt does not; the
   Windows suite sees the flag read busy.
3. **Roots and folders** — the files API lists roots and one folder each,
   ignored paths and `.git` left out; the tree draws the roots, expands a folder
   by reading it, and re-reads on each expand. AC: a companion appears as a
   second root marked read-only; `target/` is not in the tree; a path outside
   a root is refused with a sentence.
4. **Opening a file** — the read endpoint with its version and kinds; Monaco
   loaded on first open; a tab per file; an image previewed; a binary or an
   oversized file drawn as the line saying why; a read-only root opening
   read-only. AC: the chunk is not fetched until Code opens; a PNG shows as a
   picture; a file in a read-only companion takes no typing.
5. **Saving** — the dirty dot, Ctrl+S, the write endpoint over a version, the
   confirm on closing dirty, and the Reload / Keep mine bar on a refused stale
   write. AC: a save lands on disk and the dot goes; a file changed by a
   terminal between read and save is refused and the bar appears; Keep mine
   then saves.
6. **State above the pane, and maximise** — opening an Event and coming back
   keeps tabs and dirty text; leaving the page with dirty text warns; the
   maximise toggle hides the sidebar and Timeline and is remembered per device.
   AC: a swap and back finds the dirty buffer; the toggle survives a reload; on
   a narrow window the toggle is absent.
7. **The record** — ADR-0019 and the glossary already say what Code is; this
   task is the sweep for every place that still says Terminal pane: the
   vocabulary's *Avoid* lines, the Timeline's icon label, the module docs of
   `Terminal.tsx`'s successor. AC: grep for the old pane's path and title finds
   only the redirect.

## Re-verify at start

- The Terminal pane is still a details pane opened from the Timeline's header
  at `/terminal`, and `openings.ts` still lists the word-named panes by name.
- The terminals register still holds the pty handle and the child's id, which
  is what the busy check reads.
- Monaco's current release, and whether its vite integration still wants
  workers imported with `?worker` or ships a bundled loader.
- On Windows, that a file the server writes into a Worktree is readable by the
  session account — the granted entries are meant to be inherited, and this is
  the first place the server writes there.
- The vitest heap limit: a Monaco mount in jsdom may want stubbing rather than
  loading, and the suite already trips on a never-matching `getByRole`.
