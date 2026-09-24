# 01. The kind and the attachment

## What to build

A new kind of Nudge, and the watcher's life around it.

The Nudge enum in the schema crate gains one variant, carrying a Conversation
and nothing else (ADR 0009) — the page re-reads what it has expanded and what it
has open, and a re-read is cheap because a folder is one listing and a version
is one hash:

```rust
/// The Worktrees of a Conversation moved: something wrote, made, or took a
/// file away.
Files { conversation: i64 },
```

Adding a kind is the edits `docs/development.md` names — the variant, the
announce site on the server, and the row on the viewer saying which reads it
stands for. The golden fixture the viewer's own tests sweep is written from the
enum, so the kind is in both sides before anything reacts to it. Here the row is
the roots alone; the tree and the editors are tasks 03 and 04.

**The pane says it is attached by holding a socket open** for as long as it is
drawn, the way a terminal tab holds its attach — it dies with the tab whatever
becomes of the browser, so a laptop shut mid-edit stops the watcher without
anybody having to notice. A register beside the terminals' knows which
Conversations have an attachment; the first one starts a watcher for that
Conversation and the last one going stops it.

**A swap is not a detach.** The details pane is taken down whenever something
else is drawn in it, so opening an Event and coming back closes the socket and
opens another one a moment later. The stop waits out a short grace for a fresh
attachment rather than tearing the watcher down and building it again.

The watcher itself is at its simplest here — each root's own directory, nothing
below it. What makes it watch the whole Worktree is task 02.

## Acceptance criteria

- [ ] A Code pane drawn on a Conversation starts a watcher for it; the last one
      closed stops it within a moment, and a swap to an Event and straight back
      leaves the watcher running.
- [ ] A file written at the top of a Worktree by a terminal tab reaches the open
      page as one `files` Nudge, and the tree's roots re-read on it.
- [ ] The kind is in the fixture the viewer's tests sweep, and a page that has
      never heard of it reads everything back rather than nothing.
- [ ] A Conversation whose Worktree is not on disk attaches without failing, and
      watches nothing.
