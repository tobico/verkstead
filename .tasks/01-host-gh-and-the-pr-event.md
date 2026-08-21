# 01. Host gh and the PR Event

## What to build

The finish sequence stops short of the branch being reviewable, and this carries
it the rest of the way: a backlog worked to empty pushes, opens its own draft
pull request, and the Conversation moves into **Wrapping** with that PR pinned
to its Timeline.

Three pieces, and they are one slice because none of them is demonstrable
alone — the finish cannot be shown to have worked until Verkstead can find what
it opened.

**Verkstead's own reach into GitHub.** Not the sandbox's: the agents keep using
`gh` inside their sandboxes for push and PR, and this is the server running the
*host's* `gh` against a Repo and reading JSON back. It reuses whatever auth the
host already has — no token store and no GitHub App. Everything it asks about
can fail for ordinary reasons: no `gh` on the PATH, an account not logged in, a
repository with no remote, a branch with no PR yet. Each of those is an answer
to be handled rather than an error to fall over on.

**The finish that opens the PR.** The bundled next-task fork currently ends the
feature by committing the removal of `TODO.md` and saying outright not to push
and not to open a pull request. That instruction goes. In its place the fork
follows the target repository's own review process, read from its
`docs/agents/git-workflow.md` — the unstacked shape (push, then a draft PR
titled for the feature) and the stacked one (`gh stack submit --auto`, then
correcting this branch's title and body) both. It is the repository's process
rather than Verkstead's, so what the fork carries is the instruction to read and
follow it, not a copy of one project's sequence.

Nothing waits on approval anywhere in this. There is no gate in front of the
finish and none in front of the PR — merging stays the human act, and everything
up to it runs unattended.

**The PR as a pinned Event.** Once the finish step lands, Verkstead asks the
host `gh` for the PR on the Conversation's branch and records it. That is what
moves the Conversation out of Implementing and into Wrapping — a move on the
Timeline like every other. The PR joins the task list as a pinned Event: its
name and number in the Timeline, and in the details pane the commit list and the
comments, fetched rather than remembered, the same way the task list is read off
the Worktree rather than stored.

## Acceptance criteria

- [ ] A backlog worked to empty leaves a draft PR open on the Conversation's
      branch, with nothing having asked for approval at any point.
- [ ] The finish follows the target repository's recorded review process, and
      does the stacked thing on a stacked branch and the unstacked thing
      otherwise.
- [ ] Recording the PR moves the Conversation to Wrapping, and the move is on
      the Timeline.
- [ ] The PR is pinned beside the task list, showing its name and number; its
      details pane shows the commit list and comments read through host `gh`.
- [ ] A repository whose `gh` cannot answer — absent, unauthenticated, no PR
      found — leaves the Conversation where it is with the reason on the
      Timeline, rather than a Wrapping with no PR under it.
