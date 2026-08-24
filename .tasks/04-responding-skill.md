# 04. Comment batches propose before they fix

## What to build

A new bundled skill, `responding`, for the sessions the comments watcher
dispatches: given a batch of fresh comments as its feedback, the session reads
them and the code they are about, proposes what it would do about them as one
small Question Set, and on approval fixes and pushes in the same session. The
comments watcher dispatches it in place of the addressing skill, batch
semantics unchanged — one session per batch, recorded at dispatch.

A batch with nothing actionable — a question already answered by later
commits, a plain thanks — asks nothing, says so plainly as the last thing it
prints, and the batch stays settled as addressed. Spending the human's
attention only where there is a decision, as the review already does.

The same net as the review's: a responding session that ends without landing
the fixes the human accepted stops the run at an Interruption, whose retry
dispatches one fix-only addressing session handed the accepted work and the
human's words.

## Acceptance criteria

- [ ] A fresh comment batch dispatches a responding session that proposes
      before it changes anything, and fixes in place on approval.
- [ ] A batch with nothing actionable ends having asked nothing, and the batch
      settles as addressed.
- [ ] A responding session that dies holding accepted fixes raises an
      Interruption whose retry re-asks nothing.
