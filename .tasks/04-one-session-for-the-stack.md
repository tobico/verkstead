# 04. One session for the stack

## What to build

A conflict anywhere in a recorded stack sends **one** `addressing` session rather
than one per pull request, because a fix low in a stack changes everything above
it. It is told the ordered branches from the bottom, and told to sync the stack
with `gh stack sync` — running `gh stack init` over those branches first where
the worktree has no registry of its own — resolve what the sync backs out on, run
the repository's tests and push every branch.

`gh stack sync` whatever the configured resolution strategy says: a stack is
gh-stack's, and a merge into each branch of one is what the strategy's own
documentation warns against. A lone pull request is untouched — the configured
strategy, told the way it is told today, and its goes counted per pull request.

Two goes per stack, counted as each session is dispatched rather than as it ends,
and then the stop, whose Notice names the pull request left conflicting. The
count needs nothing new: it is kept against the Conversation and the Repo, which
for a stack is per stack.

**The extension is asked for before anything is sent.** `gh stack` is a separate
install, and a session's `gh` runs under a home of Verkstead's own — so the host
having the extension says nothing about what a session would see. It is asked in
the environment a session gets rather than the server's own, the way Verkstead's
own image is probed before any session is equipped with it, and a Sandbox without
it stops the run with a Notice naming the extension and the command that installs
it, rather than spending both goes on a session that cannot do what it was told.
Nothing else about the wrap-up stops with it: the checks and the merge go on
being read and written down.

**And the bundled addressing skill gets the stack's own words.** It tells a
session today to work only the branch it was sent to and never to force-push a
branch it merged into; a stack session works every branch of the stack, and `gh
stack sync` rewrites and force-pushes each of them. So a stack takes its place
beside the four kinds of feedback that skill already names — one branch and one
pull request being true of the other four and not of this one.

## Acceptance criteria

- [ ] A conflict low in a three-deep stack sends exactly one `addressing`
      session, told the ordered branches from the bottom and told to sync the
      stack, with nothing else dispatched beside it.
- [ ] Two goes and then one stop, its Notice naming the pull request left
      conflicting; a third poll dispatches nothing.
- [ ] A Sandbox without `gh stack` stops with a Notice naming the extension,
      spends no goes, and leaves the checks and the merge still being read.
- [ ] A lone pull request still gets a session under the configured strategy,
      and reaches Done once it merges clean and its checks go green.
