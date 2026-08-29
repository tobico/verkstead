# 03. Commits readable in the share

## What to build

Each commit card in the shared file opens to what the workbench's commit pane
shows: the Commit Summary rendered as the Message above the diff, and the full
syntax-highlighted diff with per-file folding. Diffs are read from git at
export time and bundled whole — **no size cap**, settled during grilling; the
"as though sitting in front of Verkstead" fidelity is the point. Commits that
landed in a read-write Companion Repo carry their repo label exactly as the
workbench draws them, and their diffs come from that companion's repository.

A commit git no longer holds (rebased away, GC'd) is bundled as the metadata
the store kept, with the diff marked unavailable rather than failing the whole
export.

Export of a long branch is where size and speed bite: the composing runs off
the request thread the way the live commit pane does, and the result must
still be a file a browser opens and scrolls.

## Acceptance criteria

- [ ] A commit card opens to message plus full highlighted diff, folded per
      file, matching the live pane's rendering.
- [ ] Companion-repo commits are labelled by repo; the work's own repo's stay
      unlabelled, as in the workbench.
- [ ] A conversation with a many-commit, many-file branch exports without
      truncation and the file still opens and navigates.
