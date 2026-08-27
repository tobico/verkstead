# 06. The vocabulary this stage adds

## What to build

The glossary catches up with what this stage built. The roadmap's own promise is
that `CONTEXT.md` gains its terms stage by stage as each lands, and this is stage
01's share of them.

**A Companion Repo entry**, written the way its neighbours are — what it is, then
the rules that matter, then an _Avoid_ line. It has to say:

- **What it is.** Another registered Repo let into a Conversation's sandbox,
  read-only or read-write. Registered, because the registry is the trust
  boundary; the Conversation's own Repo and duplicates are refused.
- **When it is settled.** Configured beside the branch while the Brief drafts,
  freely added, edited and removed, and frozen at grill start with the branch
  and the base.
- **What is checked out.** Always a Verkstead Worktree, never the human's
  checkout: detached at the selected branch's resolved commit for read-only, a
  new branch cut from the selected base for read-write. Removed at close with
  the branch kept, like the Conversation's own.
- **What a session gets.** The worktree and its repo's git directory bound by
  mode, and a neutral listing in every session prompt — the agent decides from
  the Brief what to use.

**And the two entries this changes are amended.** **Worktree** says a
Conversation has one; it may now have several — its own, and one per Companion
Repo. **Sandbox** lists what is inside and says *nothing else of the machine at
all*; that is still true, but what is inside now includes each Companion Repo's
worktree and git directory, read-only ones read-only.

Written against what the five tasks before it actually built, rather than against
what this brief imagined.

## Acceptance criteria

- [ ] CONTEXT.md holds a Companion Repo entry saying what it is, its two modes,
      when it is configured and when it freezes, what is checked out for each
      mode, what a session gets, and an _Avoid_ line.
- [ ] The Worktree entry says a Conversation may have more than one, and the
      Sandbox entry says a sandbox may hold a Companion Repo's checkout and git
      directory, by mode.
- [ ] No entry describes behaviour this stage did not build.
