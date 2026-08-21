# 01. The abandoned-roadmaps notice

## What to build

Verkstead learns to see a roadmap nobody is driving, and says so where a
Conversation is started from.

Roadmap reading today is a reading of a Conversation's **Worktree**: which
roadmap its branch has touched, and what the boxes say there. This adds the
other reading — a registered **Repo**'s roadmaps **at a commit**, with no
Worktree involved. The roadmap indexes under `docs/roadmaps/` and the stage
briefs they link to are read through git at that commit, in the Repo's own git
directory. The entry parsing, the stage type and the branch-slug rule are the
ones that already exist; only the way the bytes are fetched is new.

On top of that reading sits the rule for what makes a roadmap **abandoned**:
its next unchecked stage is **startable now**, read at the Repo's default branch
tip. All four clauses must hold.

1. Its `ROADMAP.md` has at least one unchecked numbered checkbox entry.
2. The lowest unchecked stage's brief is readable at that commit.
3. No in-progress annotation on that stage names a branch that still exists in
   the Repo. The annotation is written `*(in progress: `some-branch`)*` and the
   branch in backticks is the fact — a stale note whose branch is gone does not
   stop adoption; one whose branch is there does.
4. The stage's slug branch is not already taken in the Repo.

Clause 4 is also what keeps a stage currently mid-flight under Verkstead out of
the notice: its branch exists in the Repo's git directory even though the plan
commit that ticks its box has not reached the default branch.

Which stage is always the **lowest-numbered unchecked** one. The roadmap's order
is the roadmap's own and its stages are strictly sequential, so there is nothing
to choose. This reading has no Conversation of its own to skip, so the
skip-my-own-branch special case the settling path uses has no part in it.

The workbench draws the result under the *new conversation* box in the
conversations list: **one notice per Repo**, its abandoned roadmaps inside, each
named with its next stage. Clicking does nothing yet — task 02 is what a click
creates.

**Nothing is stored.** The list is read from the repositories every time it is
drawn, the way the pinned stage lists are, and the blocking git reads go off the
runtime's threads. A stored list would be Verkstead keeping a second opinion
about a roadmap the repository has already answered for.

There is **no dismiss control**, now or later. A true-but-unwanted notice is
silenced in the repository — tick the box, or annotate the stage.

## Acceptance criteria

- [ ] A registered Repo holding a roadmap with a startable next stage shows a
      notice naming the roadmap and that stage; a Repo whose roadmaps are all
      complete, mid-flight (annotation naming a live branch, or the slug branch
      taken) or broken (brief unreadable at that commit) shows nothing for them.
- [ ] One notice per Repo with its roadmaps inside, drawn under the new
      conversation box, read from the repositories at draw time with nothing
      stored and nothing blocking the runtime.
- [ ] Unit tests cover each clause of the rule separately against real git
      repositories, including that a plan commit which has not reached the
      default branch is invisible to a default-tip read.
