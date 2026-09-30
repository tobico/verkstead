# 03. Waiting to join

## What to build

A stage whose every task is done and whose finish is held until the chain below
it settles reads **waiting to join** on the card and in the pane, where the record
alone would have said *in progress*.

Which stages those are is already known: the server keeps a register of the ones
it is holding, and the sidebar's own *Waiting to join* label and the status
button's words are read off it at the moment a page is drawn. This is the same
register read the same way, so a stage cannot read one thing on its own row and
another on the roadmap's card.

Two consequences of it being a register of the running process rather than
something stored, and both are the behaviour rather than a shortcoming:

- **A server that has just come back is holding nothing**, so such a stage reads
  *in progress* again until the resume takes it up and finds it held a second
  time. That is exactly what the sidebar label does, and the two agreeing is the
  point.
- **Which stage it is waiting on is not said here.** The label says that the
  finish is waiting; the Notice on that stage's own Timeline says which stage of
  the roadmap it is waiting on and why. A card row has no room for the second and
  the Timeline already carries it.

*Waiting to join* wins over *in progress*: the record has such a stage in flight,
and the hold is the more particular thing to say about it. It does not win over
*halted* — a stopped Conversation is not being held by anything.

## Acceptance criteria

- [ ] A stage the server is holding before its finish reads *waiting to join* on
      the card and on the pane's heading and contents line, where task 01 would
      have read *in progress*.
- [ ] A server holding nothing reads *in progress* for the same stage, as the
      sidebar's label does.
- [ ] The card's word and the sidebar row's label for one stage are read off the
      one register and cannot disagree.
