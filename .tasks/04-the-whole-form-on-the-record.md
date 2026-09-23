# 04. The record is the whole form

## What to build

The Steer Event records everything the form settled, and the details pane
draws it as the form, frozen. Today the Event carries the target and the
instruction or follow-up brief as its body; the pairing is recorded as the
Conversation's, the companions that came in are a Notice under it, and the
digest and interrupt ticks are recorded nowhere.

The record gains the rest: the digest tick, the interrupt tick, the pairing as
picked (profile and model), and the companion rows asked for — each added
repo with its mode, base and branch, and each opened-up repo with its branch.
Kept beside the Event and keyed by it, the way a commit's repository is, and
written in the steer's own transaction. The companions Notice under the Event
stays, since it says what came in rather than what was asked. Steers recorded
before this have no such row and are drawn with the fields they have.

Every Steer Event opens now, not only one that carried a document: the details
pane draws the frozen form — the target, the body under the label the target
gives it, a line for the brief where a steer into Grilling wrote one (the Brief
itself stands under the move, as today), the digest tick, the pairing named
the way the picker names it and *a profile since removed* where it is gone,
the interrupt tick, and the companion rows. Read-only throughout, in the shape
the pending form has, so the record reads as the form the human filled. The
card keeps its line and its clamped body. The share's details pane draws the
same frozen form, and the pathing rule that skipped a steer with no document
flips: a steer always has a pane behind it.

## Acceptance criteria

- [ ] A submitted steer records digest, interrupt, pairing and the companion
      rows asked for, in the store and server suites, beside the target and
      body it records today.
- [ ] Opening a Steer Event in the workbench and in a share draws the frozen
      form with every recorded field read-only; a steer with no document opens
      too.
- [ ] A Steer Event recorded before this task opens and draws only the target
      and body.
