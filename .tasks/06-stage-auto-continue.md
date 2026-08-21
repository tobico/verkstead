# 06. Stage auto-continue

## What to build

A staged roadmap executes end to end with nobody driving it. This is the piece
that makes the whole pipeline unattended rather than merely gateless.

When a roadmap Conversation's wrap-up settles — task 04's rule — Verkstead
starts the next stage without being asked. It reads the lowest-numbered
unchecked stage out of the roadmap in the Worktree, and starts a session inside
a bundled fork of next-stage on that stage's brief. The fork re-grounds the
brief's provisional breakdown against the code as it now is, quizzes the human
on the result through `verkstead ask`, and writes `.tasks/` — at which point the
task runner from stage 03 takes over and works the backlog to empty, which
finishes and opens a PR, which wraps up, which starts the stage after it.

**The breakdown quiz is the only thing that stops the run**, and it stops it
naturally: it is a blocking ask, so the stage waits there and the Conversation
carries *blocked on you* until the human answers from wherever they are. Nothing
else in the loop asks for permission.

**A stage is a Conversation of its own.** A Conversation is one Repo, one branch
and one Worktree, and a stage is one branch and one review unit — so the next
stage is a new Conversation rather than the old one carrying on. It is created
against the same Repo, primed with the stage brief as its Brief, and it goes
straight to Implementing: the grilling that would have settled the work already
happened, and the brief is what it settled.

**The branch stacks.** Stages always stack on the unmerged predecessor, per the
target repository's recorded mechanism — the `### Stacking roadmap stages` block
under `## Review process` in its `docs/agents/git-workflow.md`. Verkstead
follows what that block says rather than carrying a stacking mechanism of its
own, and where a repository has no such block there is no convention to invent:
the stage branches off the default branch and that is said on the Timeline. The
finish from task 01 then does the stacked shape of the review process, so the
new stage's PR joins the stack.

**The roadmap keeps its own score.** The finished stage is ticked in
`ROADMAP.md` and the new one annotated as in progress with its branch name, and
that edit rides in the new stage's plan commit. So the pinned stage list moves
on its own as the roadmap executes.

A roadmap whose stages are all checked starts nothing: the roadmap is complete,
and the Timeline says so.

## Acceptance criteria

- [ ] A settled wrap-up on a roadmap Conversation starts the next stage with
      nobody asked.
- [ ] The next stage is a new Conversation on the same Repo, primed with the
      stage brief and starting in Implementing.
- [ ] The bundled next-stage fork exists, re-grounds the brief against the code,
      quizzes the human through `verkstead ask`, and writes `.tasks/`.
- [ ] The stage's branch is stacked on the predecessor per the target
      repository's recorded mechanism, and its PR joins the stack when the
      backlog finishes.
- [ ] A repository with no stacking mechanism recorded gets a branch off the
      default branch, said plainly on the Timeline rather than invented.
- [ ] The finished stage is ticked and the new one annotated in `ROADMAP.md`,
      in the plan commit, and the pinned stage list follows.
- [ ] A roadmap with every stage checked starts nothing and says it is complete.
