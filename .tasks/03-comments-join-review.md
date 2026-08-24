# 03. The review reads the pull request's comments

## What to build

Comments already on the pull request when the wrap-up session starts reach
that session as part of what it reads — whole, in the order they were said,
with where each was said — and what they ask for is folded into its one
proposal Set beside the review's own findings. Nothing said on the pull
request is acted on ungated any more: a comment's fix is proposed and
approved like any finding.

Those comments are recorded as addressed when the wrap-up session is
dispatched, the same bookkeeping the comments watcher keeps today, so no batch
session is later dispatched about them. A comment landing while the review's
Set is in flight is left alone — the Turn is held, the watcher keeps trying,
and the next batch session picks it up once the Worktree frees.

A wrap-up where the review itself finds nothing but the comments ask for work
still proposes about the comments rather than dispatching anything ungated.

## Acceptance criteria

- [ ] Comments present at review start appear in the wrap-up session's
      proposal Set, and are recorded addressed at its dispatch.
- [ ] A comment landing while the Set is in flight is not folded in, and is
      handled by a batch session after the Worktree frees.
- [ ] A review with no findings of its own but actionable comments still
      proposes, and one with neither settles by saying so.
