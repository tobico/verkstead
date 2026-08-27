# 05. Task done by checkbox

## What to build

The checkbox in `.tasks/TODO.md` becomes the source of truth for whether a task
is done — the rule stages already follow — replacing the file-is-gone signal.
Today a task whose file has not been written yet shows as completed, which is
exactly backwards while a backlog is being written.

End to end:

- The task list read by the cards and the Backlog pane takes `done` from the
  entry's checkbox, whatever files exist.
- The runner works the lowest-numbered **unchecked** entry, and reaches the
  finish step when every entry is checked (rather than when only `TODO.md` is
  left).
- An unchecked entry whose task file is missing — a hand-edited backlog, a
  crash mid-write — stops the run with a Notice naming the broken entry rather
  than dispatching a session at nothing.
- A done task's file now still exists, so the Backlog pane shows the document
  itself marked done — the way stages show theirs — instead of "Finished, and
  the document removed."

No migration: the old skill ticked the box as it deleted the file, so existing
backlogs read correctly under the new rule. This task lands before the skills
change (task 06); the reverse order would strand backlogs with files that never
disappear and boxes the server does not read.

## Acceptance criteria

- [ ] A ticked entry is done even with its file present; an unticked entry is
      not done even with its file absent — the two existing tests pinning the
      opposite flip to pin this
- [ ] While a backlog is being written, entries without files read as not done
- [ ] The runner dispatches by lowest unchecked entry, finishes when all are
      checked, and stops with a Notice on an unchecked entry with no file
- [ ] The Backlog pane shows a done task's document, marked done
