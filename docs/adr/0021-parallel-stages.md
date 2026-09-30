# Stages run side by side

Builds on [ADR-0017](0017-a-stage-knows-its-roadmap.md), which made *which
roadmap a Conversation is a stage of* a stored fact, and on
[ADR-0020](0020-a-conversation-has-a-process.md), whose stack of pull requests
is what a roadmap's stages leave behind. What this revises is the rule
`crates/server/src/stages.rs` states where it picks the next stage — *"the
roadmap's order is the roadmap's own and its stages are strictly sequential"* —
the **Stage** entry of `CONTEXT.md`, *started by the Stage before it settling*,
and two decisions the design document settled: that **the next stage starts
only after wrap-up completes**, and **Stages always stack** on the unmerged
predecessor. Both of those are refined in `docs/design/verkstead.md` as the
stages that change them land — stage 03 the stacking, stage 04 the ordering.

Decided in the grilling of 2026-09-27. The roadmap that builds it is
`docs/roadmaps/parallel-stages/`. What it is for, in the human's words: to
speed development up where possible **without compromising the review
process**, and to have enough running at once to use the cluster
(`docs/roadmaps/cluster-mode/`) when it lands — a cross-platform application
often wanting one stage per platform, a Mac one beside a Linux one beside a
Windows one.

## What was there

A roadmap's dependencies were prose in its preamble, written because the
staging skill asked for them and read by nothing. The next stage was the lowest
unticked box, started by the one before it settling, cut from that one
predecessor's branch while it was unmerged. Four guards each assumed one stage
in flight: the in-flight annotation refusing an adoption, `next_stage`
returning one stage, the refusal of a branch already taken, and the `next-stage`
skill ticking *every stage above this one*.

## A roadmap declares its dependencies

**On the stage's own line in `ROADMAP.md`**, after the link to the brief:

    - [ ] 04: The scheduler — [brief](04-the-scheduler.md) — after 01, 03
    - [ ] 01: Dependencies on the record — [brief](01-dependencies.md) — no dependencies

On the line rather than in the brief or in a block of its own, because the line
is already what the server parses, what follows the link is already kept apart
from the title, and a reader sees the stage and what it stands on in one place.

**A stage that stands on nothing says `no dependencies`.** A line with nothing
on it could be a root or could be the agent forgetting, so every line declares,
and **a roadmap declaring on some lines and not on others is refused**. The
wording is the human's: `after nothing` was offered and turned down as less
clear.

**A roadmap declaring nothing at all runs strictly in order**, exactly as
before. That is every roadmap written before this, and running those all at
once — the other reading of silence — would start stages on top of work they
were written to follow.

**A cycle, or an `after` naming a stage that is not there, is refused**: the
roadmap's own session at `verkstead done`, a roadmap already running by starting
nothing and saying why on the Timeline, and **Continue a roadmap by saying so at
the press**. That last is the one with somebody waiting for an answer — and a
roadmap written by hand or by the old tools, which is what adoption is for, is
the likeliest to declare badly — so it names the fault where the press was made
rather than on a Timeline the human has yet to open. Falling back to running in
order was rejected, because it runs a roadmap in a way nobody wrote down.

**Read afresh at every start**, off the top of the chain, so a hand edit to a
running roadmap takes effect once it is committed there. The top of a chain no
stage has joined yet is the roadmap's own branch — see *The chain* — so there is
always a branch holding the roadmap to read them off. Freezing the declarations
when the roadmap first runs was rejected: the roadmaps already in flight are the
ones most worth declaring by hand.

**A line may carry a platform**, `on windows`. It is read, recorded and shown
and nothing acts on it yet: placing a stage on a device that matches is a
follow-up once cluster mode has landed. Declared now because the wording is
cheap to settle beside the other one and expensive to retrofit into roadmaps
already written; waiting for cluster mode to design it, and holding this work
for cluster mode, were both rejected.

## What satisfies a dependency

**The stage it names settling**, not its pull request merging.
`settling.rs` already refuses to wait on the merge, for the reason that a stage
held in Wrapping until its pull request landed would hold up every stage behind
it; merging is the human's act and the roadmap does not wait on the human where
it need not.

**The server's own record says which stages have settled.** Until now the
ticked boxes did, read off the settling stage's worktree. With branches side by
side each worktree holds a different `ROADMAP.md`, so the boxes stop being one
fact. The record is ADR-0017's extended: beside which roadmap a Conversation is
a stage of, which stage, and that it settled.

**Each stage ticks only its own box, in its own finish commit.** The boxes are
the score people read. This also settles a drift: the glossary and
`stages.rs` say the tick rides in the plan commit of the stage after, the
`next-task` skill has a stage tick itself at finish, and both were happening.
Only the second survives — *tick every stage above this one* would tick a
sibling still being worked.

## What starts, and how many

**Every stage whose dependencies have settled starts**, up to two limits, both
server settings:

- **Three stages of one roadmap at once.**
- **Four Conversations across the server.** It counts every Conversation with a
  session running, of any kind, and **holds back only what Verkstead starts by
  itself** — never something the human presses. It is there for the machine:
  a stage may be a build and a test run, and the hardware is shared.

**A stage waiting on the human, or waiting to join the chain, takes a place
under both limits.** The recommendation was that it take none under the
server-wide one, nothing being running; the human chose otherwise, so a server
full of stages waiting on answers starts nothing more until one is answered.

**A stage that halts before it has joined holds up only the stages that depend
on it.** A usage limit, a failed start, a question nobody answers: every other
ready stage carries on. Stopping the whole roadmap was rejected as giving away
what the parallelism is for.

**A stage halted after it has joined holds up every later join.** Nothing joins
until the one below it has settled — see *The chain* — so a wrap-up that cannot
finish is a queue behind it, whether or not anything depends on the stage that
is stuck. **Only the join is held**: a waiting stage's own work carries on, and
its checks, its comments and its review wait on nobody. What is paid is the
order the chain is built in, and the place each waiting stage keeps under both
limits until it gets in.

**Continue a roadmap starts every ready stage**, up to the limits, the press
standing in for whatever would have started them.

## The chain

A stack is one chain, and a stage standing on two unmerged stages needs both
under it. Three ways were weighed: rebase the dependencies into one chain; hold
the stage until all but one had merged; cut from one and merge the others in.
The first was chosen — the second waits on the human at every fan-in, and the
third opens a pull request carrying other stages' commits.

**The roadmap is one chain, in the order its stages finish.**

- **The chain starts at the roadmap's own branch.** The Conversation that wrote
  the roadmap is the bottom of it while its pull request is unmerged, exactly as
  it is the predecessor today: stage 01 is cut from there, because the default
  branch does not hold the roadmap the stage is started from. Only when nothing
  unmerged is left to stand on is the default branch the base.
- **A new stage is cut from the highest settled stage in the chain**, so it
  builds on everything finished so far — its dependencies among it — and the
  rebase at its finish is small. The roadmap's own branch where no stage has
  settled yet, and the default branch where nothing unmerged is settled at all.
- **A stage whose tasks are all done waits to join.** The server holds its
  finish step until every stage already in the chain has settled. Then the
  stage rebases onto the top, opens its pull request, and wraps up.
- **One at a time**, in the order the tasks finished.
- **A stage that has joined is never rebased by a later one.**

The human's Brief had the rebase at the *start* of the stage that needs both.
Joining at the finish was chosen instead because of what it never does: **it
never rewrites a pull request somebody has started reading.** A stage is
rebased once, before it has a pull request at all. The cost, accepted, is that
two independent stages' pull requests sit one above the other and merge in
finish order.

The wait on the chain settling is the human's addition and is what makes that
true: a stage finishing while the one below is still wrapping up — a review
finding being fixed, a check going green — would otherwise rebase onto a branch
that is still moving.

**So the joins are the one thing a roadmap does in single file**, and what that
costs is worth saying plainly: a stage whose wrap-up cannot finish — an
unanswered review Set, a check that stays red, a pull request nobody can land —
holds up every later stage's join, dependent on it or not. The joining is all it
holds up, and it is the price of never rebasing a branch anybody is reading.

**A conflict at the join is the joining session's to resolve.** It resolves
it, runs the checks again, and asks the human only where it cannot tell which
side is right. Always stopping to ask was rejected.

**`gh stack` is still the session's to run and never the server's**, and how a
branch is added to a stack is still the target repository's own
`docs/agents/git-workflow.md`.

## The viewer

The stage card shows **every stage in flight**, each marked *waiting on* a named
stage, *in progress*, *waiting to join*, or *done*. It showed *done* and *to do*
and centred on the first stage not done, which with three in flight would show
one and hide two.

## What is left open

- **Placing a stage on a device by its platform**, and what happens where no
  matching device is up. After cluster mode; a grilling of its own.
- **Being told that a stage has waited long to join.** Asked about in passing
  and not taken up.
- **Several stages of one roadmap running out of one account's window**, said
  once rather than once per stage. They run under one Implementation Pairing and
  so share one usage window, and what a window running out writes today is one
  stop, one Notice, one *blocked on you* and one notification **per
  Conversation** — three stages stopping together read as three things rather
  than one. Weighed while stage 04 was planned and left exactly as it is.
