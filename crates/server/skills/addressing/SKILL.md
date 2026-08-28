---
name: addressing
description: Take one piece of feedback about work that is already on a pull request and land a fix for it. Use when a session has been dispatched with a failing check, a review finding or a pull request comment as its feedback.
---

Take the feedback in the prompt and land a fix for it. One piece of feedback,
one session, one fix: this session has none of the context of the ones that
wrote the work, and the next one will have none of yours.

The feedback is one of three things, and the job is the same for all three:

- **a check that failed** on the pull request, with what it said;
- **a finding from the review** of the branch, which the human has agreed is
  worth fixing;
- **a comment somebody left** on the pull request.

Each of them is somebody — or something — telling you that work already pushed
is not right yet. None of them is a fresh piece of work, and none of them is an
invitation to look around for others.

You start in a worktree of the repository, on the branch the work is on. The
branch is already pushed and it already has a pull request open: there is
nothing to create, nothing to switch to and nothing to open.

## 1. Find out what it is actually saying

Read the feedback whole before changing anything, then go and see for yourself.

- **A failed check**: run the thing that failed, here, and read the real
  failure. A CI log says which command failed; the repository says how to run
  it. A fix written from the log alone is a guess.
- **A review finding or a comment**: read the code it is about, and the code
  around it. What is being asked for is usually smaller than it sounds and
  occasionally larger.

If it is already fixed — a check that failed on a commit the branch has since
moved past, a comment answered by work that landed after it — say so and stop.
Committing nothing is a fine outcome; inventing a change so that something
happened is not.

## 2. Fix the cause

Fix what is wrong, not what makes the symptom go away. A test that fails because
the code is wrong is fixed in the code; a test that fails because it is testing
the wrong thing is fixed in the test, and which of the two this is, is worth
being sure about before you touch either.

**Keep to the feedback.** Anything else you notice on the way is somebody else's
piece of feedback and not this session's: a fix that also refactored two modules
is one nobody can review against the thing that was asked for.

Then run the repository's tests — the whole of the check that failed, where
there was one — and make sure what you did works before it goes anywhere.

## 3. Commit it and push it

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one.

    git add -A
    git commit -m "fix: <what the feedback was, and what you did about it>"
    git push

Push, unlike every other session here: this branch is already on a pull request,
and a fix that stays local is a fix nobody can see and nothing re-runs. The push
is what puts the commit in front of the checks again and in front of whoever
left the comment.

### What the message body says

A commit that delivers work — code, tests or documentation the work asked for —
carries a summary as its message body. That body is what the workbench shows
beside the diff when the human reviews this branch later, so it is written for
the reviewer who reads it before reading the patch. Pure bookkeeping carries
none: a plan or backlog commit, a roadmap commit, the finish commit, an ADR
recorded along the way. A commit still counts as delivering work when the list's
tick rides along with the code.

- **The prose first** — what you built and how it hangs together.
- **The diagram after it**, whenever the diff is more than three changed lines.
  The words are what the reviewer reads and the picture is what they check them
  against, so it sits under the prose and above the trailers. Diagram the
  delta rather than the system: the parts this change touches and the
  relationships between them, and nothing else. Tag each node `new`, `modified`
  or `removed` — the workbench colours those from the diff's own added and
  removed shades, so the picture and the patch read as one account of the
  change. Around ten nodes, so that it reads on a phone.

Trailers go at the end as usual; the workbench takes them off what it shows.

    feat: share the rate limiter's count between instances

    The counter moves out of the process, so every instance counts against the
    same window, and the in-process throttle it replaces goes away.

    ```mermaid
    flowchart LR
      api[POST /v1/messages] --> limiter[Rate limiter]
      limiter --> counter[(Redis counter)]
      api --> throttle[In-process throttle]

      class limiter,counter new
      class api modified
      class throttle removed
    ```

Do not open a pull request, do not touch any other branch, and do not merge
anything. The pull request exists, and merging is the human's act.

Then stop.

## When you need the human

Only when the fix genuinely cannot go on without them: feedback that contradicts
what the work was for, or a fix that would be expensive to unpick.

- **Read `verkstead guide` before the first ask**, and put the Question Set
  through `verkstead ask`. It ships inside the binary, so nothing else has to be
  found.
- **It blocks until they answer, which may be hours.** They are on a phone, not
  at this terminal, so a question printed here reaches nobody. Run the ask as a
  background command and do only work their answer cannot invalidate while you
  wait.
- **Never answer on their behalf.** If the ask itself fails — the server
  unreachable, any non-zero exit that is not a refused Set — say so and stop.
