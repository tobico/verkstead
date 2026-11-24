---
name: submitting
description: Push a finished branch and open the pull request it should already be on. Use when the work of a Conversation is built and committed but nothing opened a pull request for it.
---

The work on this branch is **already built and committed**. Whatever it was — a
backlog worked to empty, an implementation, a roadmap of stages — is done, and
the session that should have carried the branch to a pull request left none. So
that is the whole of what you are here for: push this branch, and open the pull
request the work goes for review on.

Do not start anything else. There is nothing left to plan, nothing to build,
nothing to review and nothing to improve: a session that went looking for more to
do here would put work on a branch the human is about to read as finished.

## 1. Look at what is on the branch

Read the log and the diff against the branch this one came off, and the
`git status` beside them. Two things to settle before you push:

- **What the work was**, in enough detail to title the pull request and write
  its body. The Brief in this prompt says what it was for; the commits say what
  it turned into.
- **Whether anything is left uncommitted.** There should not be, but a session
  that died on its way out may have left something. Anything that belongs to
  the work goes on the branch before it is pushed — `git add -A` and a
  `chore:` commit saying what it was tidying. That is a bookkeeping commit and
  carries no summary body.

If the branch genuinely holds nothing — no commits of its own since it came off
its base — then there is nothing to open a pull request for. Do not push an empty
branch: put that to the human as a Set with `verkstead ask`, saying so plainly.
See *When you need the human* below.

## 2. Get the branch reviewed, the way this repository does it

Read the repository's `docs/agents/git-workflow.md` — its `## Review process`,
and the `### Finish sequence` inside it — and follow that sequence step by step.
It is the repository's process rather than Verkstead's, so what is written there
is what to do, whatever another project's habits would suggest.

**The pull request itself is Verkstead's to open.** Push the branch, then open
it with one line:

    git push -u origin HEAD
    verkstead pull-request open --title '<what this built>' --body '<what the branch delivered>'

Verkstead knows where this repo's pull requests live and which token to open one
with — this prompt's *The forge this repo is on* names the forge and what it
calls a pull request — so that line works wherever the work is hosted, and it
opens a draft. Wherever the repository's own sequence says to open one with a
forge's own tool, this is what opens it instead; everything else that sequence
says — the push, the branch's name, what the body covers — is the repository's
and stands.

Two shapes, and which one this is, is a fact about the branch rather than a
choice:

- an **unstacked** branch — the ordinary case — is pushed, and then opened as a
  draft pull request titled for the work, with a summary of what the branch
  delivered as its body;
- a **stacked** branch, one made with `gh stack init` / `gh stack add`, goes
  through `gh stack submit --auto` instead, after which this branch's own pull
  request has its title and body corrected. Leave the stack's other pull
  requests alone: they belong to finished work.

**The stacked shape is on offer only where this prompt says the forge stacks
pull requests**, under *The forge this repo is on*. A forge that does not has no
stack to submit, so this branch is the unstacked shape whatever `gh stack` or the
repository's own file would say about one. Where the prompt says it does, work
out which of the two this branch is before running either — `gh stack view`
naming this branch is what says it is in a stack, and an error or a stack
without it says it is not.

**Where this prompt names a branch to open it against**, under *The branch to
open it against*, that is the base — `--base <that branch>` on the open, and
the same branch wherever the repository's own sequence asks for one. It is
there because the work was taken up off a base somebody chose rather than cut
from one, and nothing on the branch itself says which it was. Where the prompt
names none, the base is the one the Conversation was cut against and there is
nothing to pass.

**A pull request may already be half there.** The session before you may have
pushed and got no further, so a push that says everything is up to date is not a
failure — go on and open the pull request. And if the open is refused because
this branch already has one, that is the job done: say so and run `verkstead
done`.

**Nothing waits on approval.** No gate, no confirmation and nobody at this
terminal: the pull request opens unasked, and it opens as a *draft* because
merging is the human's act and nothing here is allowed to look like it was
theirs. Then run `verkstead done` — see *Saying you are done* below. Verkstead
then asks the forge for the pull request and takes the Conversation on to
wrapping it up.

**And say what happened, either way.** If you cannot open one — the push
refused, the repo's Author holding no token for its forge, the repo on no forge
at all — say what stopped you, and then run `verkstead done`. Where Verkstead
cannot reach the forge either, the signal is taken, and what you said is what
the human reads on the Timeline when Verkstead finds no pull request a second
time. Where it can, the signal is refused, and what stopped you goes to the
human as a Set instead — see *When you need the human* below.

## Saying you are done

The last thing this session does, once the pull request is open, is run
`verkstead done`. That is what ends the session, and nothing else does: not the
push, and not going quiet. So a session that waits on a push, or on an answer,
is not cut off while it waits.

Verkstead checks the repository and the forge when you run it. A refusal exits
non-zero and says on stderr what is missing — changes left uncommitted, or a
branch with no open pull request — and the session carries on: put that right
and run `verkstead done` again.

## When you need the human

Only when the pull request genuinely cannot be opened without them: a branch
with nothing on it to open one for, a push the forge refuses, or a decision about
the branch that would be expensive to unpick. Everything about *how* to open one
is in the repository's own file.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found, and it says how this backend runs an ask and what comes back from it.
- **The human answers in their own time, which may be hours.** They are on a
  phone, not at this terminal, so a question printed here reaches nobody.
  Waiting is the ask working rather than the ask failing, and how to wait is the
  Guide's — hold the ask open where it says to, end the turn where it says to —
  so either way, do only work their answer cannot invalidate.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.

## Waiting on work in the background

Before you end a turn with work of your own still running in the background — a
build, a test run — run `verkstead waiting` with how long you expect it to take,
such as `verkstead waiting 20m`, and declare it again if the work runs over.
Otherwise Verkstead takes the session as stopped and prompts it. An ask needs no
declaration. The Guide's *Waiting in the background* section has the details.
