---
name: staging
description: Write a staged roadmap under docs/roadmaps/ from settled work, and carry the branch to a pull request. Use when a Conversation's direction is a staged roadmap — read on from the grilling that settled it, or in a fresh session primed with the Brief.
---

Break the settled work into sequential **stages** under `docs/roadmaps/<name>/`,
commit them, and get the branch onto a pull request.

A roadmap is to stages what a task list is to tasks, one level up: each stage is
later one feature of its own, on a branch of its own, reviewed as a unit of its
own. What you are deciding here is where those boundaries fall.

You start in a worktree of the repository, on a branch of its own. The branch is
already made and this is already the work: there is nothing to create, nothing to
switch to, and no other plan in flight to check for.

## Where the agreement is

Two ways in, differing only in where what was settled is written down.

**You are the grilling session, reading on.** The human picked `roadmap` on your
closing Set, and the agreement is this conversation — you are the one who settled
it. Trust your own context over any summary of it, and read what they wrote
beside the pick as part of what the staging has to answer to. Step 1 is largely
behind you: you have been reading this codebase all along, so sweep for what you
have not looked at rather than starting over.

**You are a fresh session.** The grilling that settled this work is over and
its context is gone: the earlier attempt at this staging ended without a
roadmap, and the human pressed Resume. The prompt carries the Brief, and there
is no handoff document — what you have besides it is the repository. Ground the
plan in it thoroughly before you draft anything, and put what you cannot settle
from it to the human in the Set below rather than guessing.

Either way, the direction is settled and the staging is what is left.

## Why briefs rather than task files

A planning session's context is richest **now** and is gone at the next fresh
session — but detailed task slicing goes stale as earlier stages land and the
codebase moves. A roadmap splits the difference:

- **Capture now**: the decisions in force and their *why*, which are expensive
  to reconstruct later; the stage boundaries; the ordering; and a provisional
  task chunking per stage.
- **Defer to stage start**: the final task slicing, ground against the codebase
  as it exists then. Whoever starts the stage re-grounds it.

## 1. Ground the plan in the code

Read the codebase before drafting anything. Stage titles and briefs should use
the project's own vocabulary — its `CLAUDE.md` or `AGENTS.md`, its `CONTEXT.md`
glossary if it has one, the ADRs under `docs/adr/` covering what you are
touching — and the staging should match how the code is actually laid out
rather than how the plan imagined it.

If the plan's decisions are not already written down somewhere durable — a
design document, the ADRs, `CONTEXT.md` — get them written first. Briefs
*reference* decisions; they should not be the only record of them.

Where the code has drifted from what the grilling assumed, that is worth asking
about. Don't ask as you find it: finish grounding, then put the drift alongside
the stage list in the one Set below.

## 2. Define the stages

Split the effort into sequential stages. Each one must be:

- **One feature's worth of work** — a handful of tasks, one branch, one review
  unit.
- **Independently shippable** — the codebase is healthy and the trunk deployable
  after each stage merges.
- **Ordered** — later stages may depend on earlier ones. Note the genuine
  cross-stage dependencies in the index: unlike tasks, stages are sometimes
  reorderable, and the index should say which.

## 3. Put the stage list to the human

The staging is a decision they own, so it goes to them as an ordinary Question
Set — the same ordinary ask a grilling uses, and the way described under *How the
questions reach them* below.

Give the whole stage list in the Preface: each stage's title and a one-line
goal. That is the context the questions are read against. Then ask about it:
whether the boundaries and the order feel right, whether anything should be
merged, split or reordered — plus whatever drift step 1 turned up.

Iterate until they approve it. Every round is an ordinary Set and carries no
`proposal` block: the direction is settled, so there is no closing move left to
make and nothing you send ends anything. What ends this session is the pull
request below.

## 4. Write the roadmap

Pick a short kebab-case name — `mvp`, `public-release` — for the directory.
Do not rename the branch; it is the Conversation's and it is already made.

Create `docs/roadmaps/<name>/` and write `ROADMAP.md` plus one `NN-<slug>.md`
brief per stage, with zero-padded numbers.

<roadmap-template>
# <Name> roadmap

<1-2 paragraphs: what this effort realizes, linking the design or plan documents
it derives from.>

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

<Dependency notes: which stages are reorderable, which genuinely depend on
which.>

## Stages

- [ ] 01: <title> — [brief](01-<slug>.md)
- [ ] 02: <title> — [brief](02-<slug>.md)
- ...
</roadmap-template>

<stage-brief-template>
# NN. <Stage title>

## Goal

<What is demonstrable when this stage is done — end-to-end behaviour, not
layers.>

## Decisions in force

<The decisions from the grilling that bear on this stage, each with its why.
Reference design documents, ADRs and CONTEXT.md terms rather than restating
them, but DO restate rationale that lives nowhere else. This section is the
context transfer — it is what makes the stage startable in a fresh session.>

## Proposed tasks (provisional)

<A numbered draft of the breakdown: title, a one-line what, and 2-3
acceptance-criteria sketches each. Mark it clearly as provisional — whoever
starts the stage re-grounds and re-quizzes. Thin vertical tracer-bullet slices,
no file paths except decision-rich shapes.>

## Re-verify at start

<Bullet list of assumptions likely to have drifted by the time this stage
starts — things to check against the actual codebase before finalizing the
breakdown, such as "assumes X still lives in Y" or "assumes stage NN landed
first".>
</stage-brief-template>

The checkbox list under `## Stages` is what says how far the effort has got, and
it is read back by Verkstead and by whoever starts the next stage. Every stage
starts unchecked: nothing has been done yet.

## 5. Commit the roadmap

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one — the human approved the staging in step 3, and the
commit is how it gets written down.

    git add -A
    git commit -m "docs: stage the <name> roadmap"

If the planning turned up changes to `CONTEXT.md`, the ADRs under `docs/adr/`,
or other project documentation, include them in that commit: they belong on the
branch beside the roadmap that motivated them.

**Do not start stage 01.** Verkstead runs each stage as a Conversation of its
own, on a branch of its own, and a stage worked here would be a stage on the
wrong branch with no review unit of its own.

## 6. Then get the branch reviewed, the way this repository does it

The roadmap is work like any other work, so it goes for review like any other
work. Read the repository's `docs/agents/git-workflow.md` — its `## Review
process`, and the `### Finish sequence` inside it — and follow that sequence
step by step. It is the repository's process rather than Verkstead's, so what is
written there is what to do, whatever another project's habits would suggest.

Two shapes, and which one this is, is a fact about the branch rather than a
choice:

- an **unstacked** branch — the ordinary case — is pushed, and then opened as a
  **draft** pull request titled for the roadmap, with the stage list as its
  body;
- a **stacked** branch, one made with `gh stack init` / `gh stack add`, goes
  through `gh stack submit --auto` instead, after which this branch's own pull
  request has its title and body corrected. Leave the stack's other pull
  requests alone: they belong to finished work.

Work out which of the two this branch is before running either — `gh stack view`
naming this branch is what says it is in a stack, and an error or a stack
without it says it is not. A repository whose file says nothing about finishing,
or has no such file: push the branch and open a draft pull request titled for
the roadmap —

    git push -u origin HEAD
    gh pr create --draft --title '<name> roadmap' --body '<the stages it plans>'

**Nothing waits on approval here either.** No gate, no confirmation and nobody
at this terminal: the pull request opens unasked, and it opens as a *draft*
because merging is the human's act and nothing here is allowed to look like it
was theirs. Then stop — that is also what ends this session: Verkstead sees the
roadmap land, waits for you to go quiet, and takes the Conversation on to
wrapping the pull request up.

### And every companion repository you committed in

This Conversation may be working alongside other repositories — the prompt
lists them under *Companion repositories*, each with where it is and what it is
holding. Every one of them you have committed in goes the same way, and nothing
you did above carried it anywhere: `cd` into that companion's worktree, read
*that* repository's own `docs/agents/git-workflow.md`, and follow its finish
sequence there. Its process rather than this one's, on its own branch, ending
in a pull request of its own.

Only the ones holding commits. A companion you read from and never committed in
needs nothing at all, and a read-only one could hold nothing to begin with —
`git log --oneline <base>..HEAD` in its worktree, against the commit its branch
was cut from, is what says which is which.

Verkstead asks GitHub about each of them once you have gone quiet, so a
companion holding commits and no pull request stops the run rather than being
carried on past.

## How the questions reach them

Every question goes as a Question Set through the `verkstead` CLI, and nothing
else reaches anybody. There is no human at this terminal: the session runs on a
machine of its own and they answer on a phone, so a question printed here is one
nobody will ever see.

- **Read `verkstead guide` before the first ask.** It is everything the binary
  knows about asking well — how a Set is labelled, how much belongs in one, the
  shape it goes over the wire in, and how this backend runs one —
  and it ships inside the binary, so nothing else has to be found.
- **Put every round through `verkstead ask`.** The answers come back in the
  human's own time, which may be hours, and the Guide is what says how to run
  one here and what to do until they arrive. Waiting is this working rather than
  this failing, so do only work the answers cannot invalidate.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
  Approving your own staging decides in their place the very thing worth asking
  about.
