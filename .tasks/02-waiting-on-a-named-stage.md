# 02. Waiting on a named stage

## What to build

A stage of a **declaring** roadmap that has not started says which stages it is
waiting on: *waiting on 02*, and where it stands on several, **only the ones that
have not settled yet**. It replaces *to do* on the card's row, on the pane's
section heading and on the pane's contents line.

What a stage stands on is its own line's declaration, and whether each of those
has settled is the same reading task 01 put behind *done* — the server already
has both in one place, the reading that decides which stages of a roadmap may
start now. The names are the labels the roadmap's own lines carry, zero-padding
and all, because those are what the human reads the lines by.

**An undeclared roadmap's unstarted stages go on reading *to do*.** Such a
roadmap is scheduled as each stage standing on the one before it, but that is the
scheduler's reading of silence rather than something the roadmap says, and saying
*waiting on 03* about a line that declares nothing would be the viewer inventing
a declaration. An undeclared roadmap reads as it does today, with *in progress*
the one addition.

A stage whose every dependency has settled and which has still not started is not
this: it is either waiting for a place or halted for itself, and the first of
those is task 04. Until then it reads *to do*.

## Acceptance criteria

- [ ] A stage of a declaring roadmap standing on one unsettled stage reads
      *waiting on 02*, on the card and in the pane.
- [ ] Standing on several, it names only the ones that have not settled — and a
      stage all of whose dependencies have settled does not read as waiting on
      any of them.
- [ ] Every unstarted stage of an undeclared roadmap still reads *to do*.
