# 03. Commit Timeline Events

## What to build

Commits are the only visible product of unattended execution, so they become
Timeline Events. A commit landing on the Conversation's branch in its Worktree
turns into an Event summarising what changed — files touched, lines added and
removed — and the details pane shows the diff of that one commit, rendered
server-side with the folds and syntax highlighting the attached Diff already
gets.

There are **no per-commit review states**. A commit is viewable and nothing
else; feedback consolidates in the wrap-up phase in stage 04. Nothing here asks
the human for anything.

The renderer takes a raw unified diff and splits it on `diff --git`, so it needs
no structural change to take one commit's worth. It does need the right input:
fed a full `git show`, the header lines ahead of the first file would be
silently dropped rather than shown, so the commit's message has to come from the
Event itself and the diff has to arrive headerless. A repository's first commit
has no parent to diff against and still has to render.

Detection is the other half — commits arrive from a session Verkstead launched
but does not drive, so the branch has to be watched. Whatever polls the repo
shares it with a session that is committing in it, so it must not take
`index.lock`.

## Acceptance criteria

- [ ] Each new commit on the Conversation's branch lands one Commit event, in
      order, exactly once
- [ ] The Timeline row summarises the commit — its subject, files changed, and
      lines added and removed
- [ ] The details pane renders that commit's diff with the existing folds and
      highlighting
- [ ] A root commit renders
- [ ] Polling the repo never blocks or trips a session's own `git` commands
