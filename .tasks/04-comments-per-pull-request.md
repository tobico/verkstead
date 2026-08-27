# 04. Comments per pull request

## What to build

What is said on any of a Conversation's pull requests is read, and answered by
a session that knows which one it is answering.

**A watcher per recorded pull request**, the shape of the checks watchers
beside it: it asks GitHub what has been said on that pull request, in that pull
request's own repository, on the same interval. All three places a human writes
still count — the conversation, the words at the top of a review, and the
comments left on the lines of the diff.

**The bookkeeping is per pull request.** Which comments have been dispatched
about is kept per Conversation today; it gains the repository, so that what is
settled is *this pull request has nothing outstanding* rather than a single
answer covering all of them. Which is what lets one pull request go quiet while
another is still being answered.

**The review is given everything standing on every pull request when it
starts.** The capture that hands the review what was said already reads
whatever is unaddressed and records it as addressed in the same breath, so that
no batch session is later sent to do what nobody agreed to. It reads across all
of them now, and each comment says which pull request it was left on — *this is
the wrong way round* is an instruction with the repository and the line and a
riddle without.

**The batch session is told which repository, which pull request and where to
work**, exactly as a fix session is: the feedback names them, and the
responding skill is written for a branch and a pull request that may be a
companion's — its reading of the diff, its push, and its rule about not
touching any other branch.

**One batch at a time, still.** A session takes the Conversation's Turn, so
comments on two pull requests are answered one after the other rather than by
two agents in overlapping worktrees. A batch that grew while the watcher waited
is one session about more of what was said, which is what a batch is for.

**Done waits for every pull request's comments.** The comments settle per pull
request, the way the checks do, and the rule that ends a wrap-up expects a
settled comments row for every pull request on the record.

## Acceptance criteria

- [ ] A comment on a companion's pull request dispatches one batch session that
      works in that companion's worktree and pushes there, and the feedback
      names the repository and the pull request.
- [ ] The review is given the comments standing on every pull request when it
      starts, each said with the pull request it was left on, and no batch
      session is later dispatched about any of them.
- [ ] One pull request going quiet settles only its own comments; a Conversation
      reaches Done only once nothing is outstanding on any of them.
- [ ] A Conversation with one pull request behaves exactly as it does now, and a
      database written before this carries its addressed comments across.
