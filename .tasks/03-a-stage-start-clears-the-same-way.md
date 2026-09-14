# 03. An unattended stage start clears the same way, and the decision is written down

## What to build

A stage started by the stage before it settling cuts its worktree with nobody at
a button. Give it the clearing task 02 built: once its own worktree is made and
before the record moves, a `.tasks/` inherited from the branch it stands on is
removed and committed as the configured git author, with the same message and
the same Timeline notice.

Where no git author is configured, halt the stage before anything is made — no
branch, no worktree, no companions — and say so on the Timeline the way a fetch
git would not make already halts one:

> Stage NN of the `<roadmap>` roadmap is next, and no git author is
> configured, so Verkstead cannot commit on its branch. Nothing was started.
> Set one in Settings and continue the roadmap from there.

The devices are told as a stage that started would tell them, because a notice
on a Timeline nobody has open reaches nobody.

Then write the decision down where the human asked for it. The module docs of
the worktree cut, the starts and the unattended stage start say that Verkstead
now edits a worktree it cut, why (the watcher reads the tree, not the branch's
history), and what it refuses for. The Worktree entry in `CONTEXT.md` gains a
paragraph saying an inherited task list is cleared at the cut, as a commit by
the configured author, and that every cut for new work needs one. No ADR.

The tests sit beside the existing settle-and-continue tests: a predecessor
branch still carrying a list, and a settle with no author configured.

## Acceptance criteria

- [ ] A stage cut from a predecessor branch whose tip still carries `.tasks/` starts on a tip with no `.tasks/`, with the clearing commit by the configured author and the notice on its Timeline.
- [ ] A settle with no git author configured starts no stage, makes no branch or worktree, and puts the notice above on the settled Conversation's Timeline.
- [ ] `CONTEXT.md`'s Worktree entry and the module docs describe the clearing, the refusal and the deletion-is-not-a-writing reading.
