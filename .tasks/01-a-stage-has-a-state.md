# 01. A stage has a state

## What to build

The stage list the pinned card is drawn from, and the roadmap's details pane,
carry **a state per stage** in place of the checkbox alone — and the state is the
server's own reading rather than anything the viewer works out. Both places say
the state in the words the human reads: on the card's row where *done* and *to
do* were, on the pane's section heading where the same two words were, and on
the pane's table-of-contents line, which says nothing about a stage today.

This task is the shape and the three states that come straight off Verkstead's
record of a stage Conversation:

    done | in progress | halted | to do

- **done** — the record says the stage settled, or it holds no row for the stage
  and its box is ticked. This is the rule the scheduler already spends on every
  stage of a roadmap, and it is the same rule here: a stage that settled is done
  however the box reads on the branch being drawn, because each branch carries a
  `ROADMAP.md` of its own.
- **in progress** — the record has the stage in flight and nothing else is true
  of it. New: there was no such word before, so this is what a roadmap being
  worked in order now says about the stage somebody is on.
- **halted** — the stage's Conversation has stopped, or the record says it was
  abandoned: closed without ever having wrapped up. One word for both, because
  what a reader does about either is the same — go and look at that
  Conversation — and it is *not* in progress, which is the distinction worth
  drawing. A halted stage holds up only the stages that stand on it.
- **to do** — everything else, for now. The three waits that replace it are the
  three tasks after this one, each adding its own state to the list above.

A roadmap the record holds no rows for is read off its boxes alone and comes out
*done* or *to do* throughout, exactly as it reads today. That is every roadmap
worked by hand or by the old tools, and drawing it differently would be this
change leaking into work it knows nothing about.

The card is read off the Worktree today and knows nothing of the record, so the
record has to reach it: the reading that builds the card and the one that builds
the pane both gain it, and both are already called from handlers that hold the
pool. One reading rather than two — the card is pinned on every stage's Timeline
and on the roadmap's own, and it says the same thing about a stage on each.

## Acceptance criteria

- [ ] The stage list an Event carries and the roadmap pane both carry a state per
      stage rather than a box alone, and the card, the pane's section heading and
      the pane's contents line all say it.
- [ ] A stage the record has in flight reads *in progress*; one whose Conversation
      has stopped, and one the record says was abandoned, both read *halted*.
- [ ] A stage that settled reads *done* however its box stands on the branch being
      drawn, and a roadmap the record holds no rows for reads off its boxes
      exactly as it does today.
- [ ] The card says the same thing about a stage on the roadmap's own Timeline as
      on that stage's.
