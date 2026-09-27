# 02. The narrowed wrap-up

## What to build

A Fix Merge Issues Conversation's wrap-up waits on **Mergeable** and the checks
and on nothing else: no review session is ever run, and nothing said on the
pull request ever dispatches a `responding` session.

**Two halves, and both are needed.** The entry settles **Review** and
**Comments** for the recorded pull request as it lands in Wrapping — the
Resolve-conflicts press's shape widened, that press already entering with the
review's settle standing and Mergeable unsettled — written in or before the
transaction that makes the move, so no sweep or restart can find a wrapping
Conversation that has not got them yet. That is what lets the wrap-up's own
rule reach Done, since the rule waits on the review and on every recorded pull
request's comments.

Settling alone does not hold, which is the second half. The review's settle is
taken by hand on a steer into Wrapping, and the comments watcher *unsettles*
Comments the moment anything lands on the pull request — settling Review is
precisely what tells that watcher comments are now a batch session's to act on.
So the review watcher and the comments watcher each read the Conversation's
Process for themselves and do nothing for this one, which is the pattern the
wrap-up already follows: nothing that starts the watchers decides what runs,
each of them asks the record a moment later. Read that way, every door into
Wrapping behaves alike — the start, a restart, Resume, the Resolve-conflicts
press and a steer — with no bookkeeping per door.

What goes on running untouched is the rest of it: the companions covered, the
checks watched and fixed, the mergeability read, and the rule that ends the
whole thing. Over a bare branch the settles are the same, the pull request
being opened into the same repository a moment later.

**And the Timeline says so.** The note the take-up already writes about what
was taken up also says, for this Process, that the wrap-up is narrowed: no
review will be read and nothing said on the pull request will be answered.

## Acceptance criteria

- [ ] A Fix Merge Issues Conversation entering Wrapping has Review and Comments
      settled for its pull request, and no review session is started — over a
      named pull request and over a bare branch both.
- [ ] A comment landing on that pull request dispatches nothing, unsettles
      nothing, and leaves the Conversation free to reach Done.
- [ ] Neither a Resume nor a steer into Wrapping makes a review session run on
      one.
- [ ] A failing check still dispatches a fix session as it does today, and the
      Conversation reaches Done once the checks are green and GitHub says the
      pull request merges — with no review and no `responding` session anywhere
      on its Timeline.
- [ ] The take-up's Timeline note says the wrap-up is narrowed, and says it for
      this Process alone.
