# 01. A deletion under `.tasks/` is not a plan written

## What to build

The runner has one reading for whether a branch has written a backlog since its
base commit, asked by the stage-to-plan check and by Resume. Today it counts
*any* commit touching `.tasks/` since the base, and *any* pending change under
it, as a plan written. That reading was right while nothing but a session ever
touched the directory. Task 02 has Verkstead commit a removal of an inherited
list before any session runs, and after that a planning session that dies before
its plan commit would leave a branch whose only `.tasks/` history is a deletion:
read as planned, with nothing to work, and Resume refusing it by name. That is a
stage stuck for good.

Teach the reading that a deletion is not a writing. In history, count only the
commits since the base that add or change a file under `.tasks/`. In the
Worktree, count only pending additions and modifications, never a staged or
unstaged deletion. A branch whose whole `.tasks/` story since the base is
removal reads as never planned.

Keep every answer the reading gives today for the shapes it already knows: a
backlog written and worked to empty (the finish took `.tasks/` away, but the
plan commit added it, so still *written*), a session that wrote a list and died
before committing (*written*), a stage that never planned (*not written*), and a
repository that will not answer (*written*, the careful way round).

Unit tests sit beside the existing ones for this reading. Add the two new
shapes: a removal commit as the only history, and a staged deletion with nothing
committed.

## Acceptance criteria

- [ ] A branch whose only `.tasks/` history since the base is a commit removing the directory reads as *not written*, so a stage on it is planned rather than stuck.
- [ ] A Worktree with a staged deletion of `.tasks/` and no commit touching it since the base reads as *not written*.
- [ ] Every existing test of the reading still passes: a finished backlog, a dead planning session's uncommitted list, a never-planned stage and an unreadable repository each give the answer they give today.
