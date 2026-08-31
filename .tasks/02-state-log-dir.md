# 02. The state/log directory, and the term for it

## What to build

The second directory of Verkstead's own outside the **Data Directory**,
resolved beside the first: `~/.local/state/verkstead` on Linux,
`~/Library/Logs/Verkstead` on macOS, `%LOCALAPPDATA%\Verkstead` on Windows.
Stage 02's desktop binary writes the server's log file there, because the
stdout of a tray app launched from an icon goes nowhere.

**Nothing in this stage writes to it, and nothing creates it.** What this task
delivers is the answer to where it would go, exported from the server crate so
the desktop crate can call it. The binary that opens a file is the one that
makes the directory — the Build Cache is the precedent for a directory
Verkstead creates outside its own, and it creates it where it uses it.

Same shape as task 01's resolution and for the same reason: the environment
values in, a path out, every platform arm exercised by unit tests on the Linux
runner rather than only the one this CI compiles. Linux is XDG again, so
`$XDG_STATE_HOME` where it is set and absolute and `~/.local/state` otherwise.
Worth noticing while writing it that the three platforms disagree about what
this directory even *is* — a state directory on Linux, a logs directory on
macOS, the local rather than roaming application data on Windows — so it is one
helper with three arms, not a general notion with three spellings.

**Nowhere to resolve to answers nothing**, rather than refusing anything.
Unlike the Data Directory nothing at startup turns on this: the server keeps
logging to stdout whatever the answer, and the only caller that would open a
file arrives in stage 02. What a desktop binary with no home should do is that
stage's decision, made where there is a dialog to put it in.

**And the term.** CONTEXT.md's **Build Cache** entry is the pattern to follow —
a directory of Verkstead's own that is not the Data Directory, written in terms
of what it is for and where it is. This one wants the same treatment, including
that it stands empty and uncreated until there is a desktop binary to write in
it, so that a reader looking for where the app's logs go finds the answer in
the vocabulary rather than in the code.

## Acceptance criteria

- [ ] The resolution answers the platform state/log directory on each of the
      three platforms, every arm covered by a unit test that runs on the Linux
      CI, and honours an absolute `$XDG_STATE_HOME` on the Linux arm as task 01
      honours `$XDG_DATA_HOME`.
- [ ] It is callable from another crate in the workspace; nothing writes there,
      no directory is created, and nowhere to resolve to answers nothing rather
      than failing anything.
- [ ] CONTEXT.md carries an entry for it written as the **Build Cache** entry
      is, and a reader looking for where the desktop app's logs go finds it
      there.
