# 01. Reading the Repo

## What to build

Both runners read committed files: the backlog runner `.tasks/TODO.md` and the
`NN-<slug>.md` names beside it at the Worktree's HEAD, and the stage runner a
roadmap's `ROADMAP.md`, its briefs and `docs/agents/git-workflow.md` at the top
of the roadmap's chain. No host function lets an Activity do that today, and
the `filesystem` capability is a fixed host path, so it cannot reach a Worktree.

Add host functions to the `activity` world's `host` interface (WIT
`crates/server/wit/plugin.wit`) that read **committed bytes, never the working
tree**:

- read a file of a named Repo (the Worktree's or a Companion's, named as
  `git-facts` names them) at a revision — a branch, a commit, or the Worktree's
  HEAD — answering its contents, none where nothing is there, or a refusal in a
  sentence;
- list a directory of the same at a revision, answering the entries' names.

Read through `git show` / `git ls-tree` with `--no-optional-locks`, as
`repos::git` already does, so a read never trips a session's own `git add`.
**No capability is declared for it** (Set 1839, Q2): it is the Conversation's
own Repo, on the same footing as `git-facts`. Say so in the WIT doc comments
and in ADR-0030's *Activity* section.

The stage runner will need the Repo's git directory at a branch nothing has
checked out (the chain's top), so the revision is resolved in the Repo, not
only in the Worktree.

## Acceptance criteria

- [ ] A fixture Activity, hosted through the suite's `activities::Hosting`
      seam, reads `.tasks/TODO.md` at HEAD and sees a box ticked only once the
      tick is committed, not while it is merely written
- [ ] It lists `.tasks/` and reads a file at a named branch the Worktree does
      not hold
- [ ] An unknown Repo name, a path escaping the Repo, or an unknown revision is
      refused in a sentence, and none is answered for a path that is not there
- [ ] The follow-up rounds component still builds and its tests pass
