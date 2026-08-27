# 06. Skills keep task files

## What to build

The shipped task skills stop deleting task files one by one. A task is reported
done by ticking its entry in `TODO.md` and committing; the file stays where it
is. The finish step — reached when every entry is checked — removes the whole
`.tasks/` directory, task files and `TODO.md` together, in the finishing
commit.

This touches the skills Verkstead ships and installs (the next-task skill above
all, and every passage in the task skills that states the file-is-gone rule —
the breaking-down and next-stage skills describe it too). The wording of the
done-signal changes wherever it is written: what says a task is done is its
box, and what says the feature is finished is every box ticked.

Lands after task 05: the server must read checkboxes before the skills stop
deleting files.

## Acceptance criteria

- [ ] Working a task ticks its entry and deletes nothing; the commit carries
      the tick beside the code
- [ ] The finish step removes `.tasks/` entirely in the finishing commit
- [ ] No skill text still states the file-is-gone rule as the done-signal
