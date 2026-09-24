# 02. The watcher walks the Worktree

## What to build

The watcher of task 01 grows from the roots' own directories into the whole of
each Worktree, and learns not to shout.

**Ignore-aware, because the alternative exhausts the machine.** Every
non-ignored directory under each root is watched, walked by the same ignore
rules the tree already hides by — git's own answer rather than a second
implementation of it — and `.git` is not among them. The recommended watcher on
Linux takes a watch per directory, so a recursive watch over a Rust `target/`
would spend a machine's whole inotify allowance on files nobody can see. A root
git will not answer about is walked whole, the way the tree lists one whole.

**And directories are added as they appear**, so a folder made after the watcher
started is watched without anything being restarted.

**Debounced to one Nudge per burst.** A build writes thousands of files and the
page wants to look once: what leaves the watcher is one Nudge for a burst of
writes, and a quiet Worktree still reaches the page promptly.

**And each root's git `index` and `HEAD` beside the tree**, and nothing else
under a repository's insides — the commit's own writes are the Worktree moving
too, and they are what the marks of task 05 depend on. A Worktree's `.git` is a
*file* pointing at the repository's `worktrees/<name>` directory, so those two
are found by asking git where its git directory is rather than by joining `.git`
onto the root. A commit made on a branch rewrites the index without moving HEAD,
which is why the index is the one that catches a commit; HEAD is what catches a
checkout.

## Acceptance criteria

- [ ] A file written three levels down in a Worktree produces one `files` Nudge.
- [ ] A `cargo build` in a terminal tab produces a handful of Nudges rather than
      thousands, and the page's reads are the handful rather than the build.
- [ ] A Worktree carrying a large ignored directory starts watching in under a
      second, and nothing inside that directory is watched.
- [ ] A folder made after the watcher started is watched, and a file written in
      it produces a Nudge.
- [ ] A commit made in a terminal tab reaches the page, and so does a checkout.
