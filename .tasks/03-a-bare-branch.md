# 03. A bare branch

## What to build

**A Target that is neither a URL nor a `#number` is a branch, and it is taken up
the same way with no pull request at the end of it.** The work is built and
pushed and nobody opened one; the wrap-up is still the whole of what there is to
do.

**The same take-up.** Fetch, and then the branch settled against origin by the
rules a pull request's head is settled by: no local branch is cut tracking
origin's, one standing behind origin's is fast-forwarded on to it, one level with
origin's is checked out where it stands, and one that is ahead, has gone its own
way, or is checked out somewhere else is refused by name, naming the place. A
name origin has nothing under is refused by name as well — a branch that is not
on origin is nothing to wrap up, there being nowhere for a review to happen. The
companions come along exactly as they do for a pull request.

**The base is the picker's.** The base commit is the branch's head at take-up, as
it is for a pull request, so that the Timeline draws only what Verkstead adds
from here. What differs is the name beside it: GitHub has no base to give, so
what is recorded is the branch the base picker holds — which is what the pull
request will be opened against.

**Wrapping is entered by a move of its own.** Recording a pull request is the
door every other ending comes through, and there is no pull request here, so the
take-up makes the move itself: the branch, the base pair, the worktree and the
state in one transaction, with the move on the Timeline. Nothing about a pull
request is written, and the Timeline draws only what Verkstead adds.

**Then one session is sent for the pull request.** The same `submitting` session
that a run which stopped short of its push is sent — the work is already built,
and opening the pull request is the whole of the job — **told which branch to
open it against**, because the skill's own fallback opens against the
repository's default branch and the base here is the one the human picked. It
opens a draft, as it always does: merging is the human's act.

**And what it opens is recorded without a second move.** The Conversation is
already Wrapping, so the pull request is written beside it the way a companion's
is, and the wrap-up's watchers are started on it — the checks, the comments, the
review over a branch nobody has read, and the rule that ends the whole thing. A
session that opens none stops the run with what it last said on the Timeline,
exactly as one that stopped short of its push does, and Resume is another go.

A Review naming a pull request is untouched by any of this: it has GitHub's base,
its pull request is recorded at the press, and no `submitting` session ever runs
for it.

## Acceptance criteria

- [ ] A Review whose Target is a branch on origin lands Wrapping with that branch
      checked out, the head at take-up as its base commit, the picked base as the
      branch it came off, and no pull request recorded.
- [ ] A branch origin has nothing under is refused by name on the composer, and
      so are one that is ahead, one that has diverged and one checked out
      elsewhere.
- [ ] One `submitting` session runs in the worktree and opens a draft pull
      request against the picked base rather than the repository's default
      branch.
- [ ] The pull request it opens is recorded against the Conversation and the
      wrap-up's watchers run on it; a session that opens none stops the run with
      what it last said.
- [ ] A Review naming a pull request still lands Wrapping at the press with no
      session sent.
