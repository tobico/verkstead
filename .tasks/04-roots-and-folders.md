# 04. Roots and folders

## What to build

The files API, and the tree that stands on it.

**Conversation-scoped under `/api/ui/`, beside the terminals' routes** — which
is what puts it behind the Workbench Key, and the key is what keeps a session
out of it (ADR-0019, *The server reads and writes the Worktree, outside the
Sandbox*). The server reads as itself, with no Sandbox in front of it; what
bounds it is the roots.

**The roots are the Conversation's own Worktree and each companion's**, in the
order the Conversation carries them, each saying whether it is writable. The
module that composes a Set's Diff lists the writable worktrees and is the
*pattern* rather than the code: it drops read-only companions entirely, and
Code wants them as roots marked read-only.

**A folder listing is one folder**, read when it is expanded and never a walk
— the path field's directory browse is the shape. Git-ignored paths and `.git`
are left out, and git is asked about the ignores rather than having them
reimplemented.

**Refusals are sentences in the body, the way registering a Repo refuses**: a
path outside a root, a path under `.git`, and a root that is no longer there
are each a different thing for a human to read, none of them a status code.

The tree draws a root per Worktree down the side of the pane, read-only ones
marked, and expanding a folder reads it — again on every expand, because
nothing yet tells the page the disk has moved. No row menu, no quick open and
no git status marks: those are stages 03 and 04 of the roadmap.

**Windows**: a file the server writes into a Worktree has to be readable by the
session account, and the granting module writes one entry that inherits, so it
should be. Wine does not emulate ACL inheritance, so what the suite asserts is
the inheritance flags on the written entry; the effective read stays a manual
check on a real machine, noted where the grant is written.

## Acceptance criteria

- [ ] The roots endpoint lists the Conversation's own Worktree first and each
      companion after it, a read-only companion marked read-only, and the tree
      draws them.
- [ ] Expanding a folder reads that folder alone, and `target/` and `.git` are
      not in the tree.
- [ ] A path outside a root, and a path under `.git`, each come back as their
      own sentence rather than as a status code.
- [ ] The Windows suite asserts the inheritance flags on the entry granted over
      a Worktree.
