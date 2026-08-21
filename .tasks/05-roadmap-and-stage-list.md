# 05. Roadmap direction and the stage list

## What to build

The third Direction stops being refused. Roadmap is the one the chooser draws
and then turns down — `RoadmapNotYet` — because there was nothing to start. This
gives it something.

**Choosing Roadmap writes a roadmap.** It starts a session under the
implementation Profile inside a bundled fork of to-roadmap, primed with the
Brief and the handoff the grilling settled, exactly as the other two Directions
are. What it produces is `docs/roadmaps/<name>/` in the Worktree: a `ROADMAP.md`
listing the stages as a checkbox list, and a brief per stage carrying the goal,
the decisions in force, a provisional task breakdown and what to re-verify when
that stage starts. It commits what it writes, and the Conversation is
Implementing while it does — writing the roadmap *is* this Conversation's work.

The file formats are the repository's, not Verkstead's. They are the same ones
`/to-roadmap` already writes and `/next-stage` already reads, because task 06's
fork has to consume what this one produces, and a roadmap a human wrote by hand
has to be readable too.

**The stage list is pinned.** Verkstead reads `docs/roadmaps/` back out of the
Worktree and draws it as the pinned stage-list Event, beside the task list and
the PR. Nothing about it is stored: it is a reading of the repository as it
stands, the same way the task list is, so it cannot disagree with the branch it
is read off. The entries are the stages in their order, with their titles, and
which of them are checked.

A Worktree with no `docs/roadmaps/` in it pins no stage list, the same way one
with no `.tasks/` pins no task list. So does one whose roadmap cannot be parsed
— there is nothing for the human to do about either.

## Acceptance criteria

- [ ] Choosing Roadmap is accepted and starts a session; `RoadmapNotYet` is
      gone from the chooser and from the code behind it.
- [ ] The bundled to-roadmap fork exists and writes `docs/roadmaps/<name>/` —
      a `ROADMAP.md` and a brief per stage — and commits it.
- [ ] The formats written are the ones `/to-roadmap` and `/next-stage` already
      use, so a roadmap written by either is readable by the other.
- [ ] The stage list is pinned beside the task list and the PR, showing each
      stage's number, title and whether it is checked.
- [ ] A Worktree with no roadmap, or an unreadable one, pins no stage list.
