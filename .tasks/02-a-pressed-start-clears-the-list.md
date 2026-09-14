# 02. A start a human presses clears the inherited list

## What to build

Three presses cut a worktree for new work and land a session in it: a grilled
start, an ungrilled build, and Continue a roadmap. Give all three the same step
between the cut and the record moving: where the Conversation's own worktree
holds a `.tasks/` directory, remove it and commit the removal on the fresh
branch as

    chore: clear the task list inherited from <base branch>

authored on the command line as the configured git author, the way Create repo
commits its README, so nothing is written into the repository's config and no
trailer is added. The recorded base stays the commit the branch came off; the
clearing commit sits after it, and the Timeline's commit sweep draws it as a
commit of the Conversation like any other. Companion worktrees are untouched:
nothing reads `.tasks/` off a companion.

Every one of the three presses refuses by name, before any branch or worktree is
made, where no git author is configured — whether or not the base carries a
list. The human decided the author is not optional: onboarding collects one, so
a start without it is a misconfiguration to name rather than a case to work
around. The refusal is checked in the order the others are, after the record,
the Profiles and the Brief and before anything that costs a git call. It is a
new refusal on the start and the adoption answers, drawn on the compose page and
the adoption page as:

> No git author is configured, so Verkstead cannot commit on the branch. Set
> one in Settings before starting work.

Where a list was cleared, put a notice on the Timeline, every time, as:

> The base branch `<name>` carried a task list, **<heading>**, with N of M
> entries still open. It was cleared before this work started.

The heading is the list's own `TODO.md` heading and the counts are its entries
and the unticked ones among them; a list with no heading or no entries still
gets the notice, with what it has. A `git rm` or commit that fails unwinds the
checkout and the branch and refuses the way a worktree git would not make
already refuses.

The planning watcher needs no change once the tree is clean: `.tasks/TODO.md`
is gone from the tip, so the landing it waits for is the session's own plan
commit.

Integration tests go beside the existing start and adoption tests: a base that
carries a committed list, a base that does not, and a start with no author.

## Acceptance criteria

- [ ] Starting a grilled Conversation, an ungrilled build, or an adopted stage from a base whose tip carries `.tasks/` gives a branch whose tip has no `.tasks/`, one commit by the configured git author with the message above, and the notice on the Timeline.
- [ ] Starting from a base with no `.tasks/` makes no extra commit and no notice.
- [ ] With no git author configured, each of the three presses is refused by name with the wording above, and no branch or worktree exists afterwards.
- [ ] A planning session launched on a cleared branch is not ended by the watcher until it has committed a list of its own.
