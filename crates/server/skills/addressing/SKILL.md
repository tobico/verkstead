---
name: addressing
description: Take one piece of feedback about work that is already on a pull request and land a fix for it. Use when a session has been dispatched with a failing check, a review finding, a pull request comment or a merge conflict as its feedback.
---

Take the feedback in the prompt and land a fix for it. One piece of feedback,
one session, one fix: this session has none of the context of the ones that
wrote the work, and the next one will have none of yours.

The feedback is one of four things, and the job is the same for all four:

- **a check that failed** on the pull request, with what it said;
- **a finding from the review** of the branch, which the human has agreed is
  worth fixing;
- **a comment somebody left** on the pull request;
- **a merge conflict** between the pull request and its base branch, which
  nobody has touched the branch to cause and which nothing can land over.

Each of them is somebody — or something — telling you that work already pushed
is not right yet. None of them is a fresh piece of work, and none of them is an
invitation to look around for others.

You start in a worktree of the Conversation's own repository, on the branch the
work is on. That branch is already pushed and it already has a pull request
open: there is nothing to create, nothing to switch to and nothing to open.

**Work where the feedback says.** This Conversation may hold a pull request in
more than one repository — its own, and one per companion repository the work
committed in — so feedback about one of them names the repository, the pull
request and the worktree to work in. `cd` into that worktree first and do the
whole job there: `git` and `gh` both read their repository from wherever they
are run, so a `gh pr checks` from the wrong directory asks about somebody else's
pull request. Feedback that names no worktree is about the branch you started
on, and that is where to work.

Whichever it is, it is one branch and one pull request: the one in the worktree
you are working in, already pushed and already open.

## 1. Find out what it is actually saying

Read the feedback whole before changing anything, then go and see for yourself.

- **A failed check**: run the thing that failed, in the worktree the feedback
  named, and read the real failure. A CI log says which command failed; the
  repository says how to run it. A fix written from the log alone is a guess.
- **A review finding or a comment**: read the code it is about, and the code
  around it. What is being asked for is usually smaller than it sounds and
  occasionally larger.
- **A merge conflict**: fetch first, so the base you work against is the one
  GitHub is looking at, then start what the feedback asked for — a merge or a
  rebase — and let git show you where the two sides disagree. **The feedback
  names which of the two**, because they are different acts on the branch and
  the human has configured which one this repository gets: do the one you were
  told to do, and neither the other nor both. Read both sides — this branch's
  change and what has landed on the base since it parted — before you write
  anything, and read what each was for rather than only what each says. A merge
  that comes through clean is still a merge to commit and push: it is what puts
  the base's work on the branch, and it is what GitHub reads next.

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

**A conflict is two changes to reconcile**, and resolving it means keeping both.
Taking one side's hunk wholesale — `--ours`, `--theirs`, or the same thing done
by hand — is not a resolution: it throws away work somebody did, and it throws
it away silently, because the merge then looks exactly like one that went
cleanly. Write what the two changes together were meant to do. Where they
genuinely cannot both stand, that is a question for the human rather than a call
to make on their behalf — see *When you need the human* below. And do not undo
it to escape it: a `git merge --abort` or a `git rebase --abort` leaves the pull
request exactly as conflicted as it was.

Then run the repository's tests — the whole of the check that failed, where
there was one, and the whole suite after a merge — and make sure what you did
works before it goes anywhere. A merge that compiles is not a merge that
reconciled anything.

## 3. Commit it and push it

**Nothing waits on approval.** There is no gate, no confirmation and nobody at
this terminal to ask for one.

    git add -A
    git commit -m "fix: <what the feedback was, and what you did about it>"
    git push

In the worktree the feedback named, which is where the fix was made: those three
are one repository's, and a commit made in the wrong directory is a change to
work nobody asked about.

A resolved conflict commits the same way — the merge left everything staged, so
`git add -A` and `git commit` are what finish it, and a rebase finishes each
conflicted commit with `git add -A` and `git rebase --continue`.

**How it is pushed is the feedback's to say, and only the feedback's.** A merge
is pushed as it stands and **never force-pushed**: the branch keeps every commit
it had, so nothing anybody has already read moves and nothing stacked on it
breaks. A rebase has rewritten the branch and cannot be pushed any other way, so
the feedback that asked for one asks for `git push --force-with-lease` — the
lease being what stops the push landing over work that arrived while you were
resolving. Never force-push a branch you merged into, and never plain-push one
you rebased: each of those is the other strategy's ending, and one of them
fails while the other quietly undoes what was asked for.

**The commit that resolves a conflict is bookkeeping**, so it carries no message
body under the rule below: what it puts on the branch is the base branch
arriving rather than anything you set out to build, and its diff against either
parent is the other parent's work. Say what you reconciled in the subject line
and leave it at that.

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

Do not open a pull request, do not merge anything, and do not touch any branch
beyond the one you were sent to: the branch checked out in the worktree the
feedback named, or the one you started on where it named none. Every other
branch — in this repository and in every companion beside it — belongs to
somebody else's piece of feedback. The pull request exists, and merging is the
human's act.

Then stop.

## When you need the human

Only when the fix genuinely cannot go on without them: feedback that contradicts
what the work was for, or a fix that would be expensive to unpick.

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
