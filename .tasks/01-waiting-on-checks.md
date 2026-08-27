# 01. Draw the narrowed wrap-up as Waiting on checks

## What to build

A Wrapping Conversation whose wrap-up has narrowed to its checks — the review
and comments facts settled, checks not, and no session running in its
Worktree — reads **Waiting on checks** on its card and in the sidebar, and a
Notice lands on its Timeline the moment it narrows.

This is a **derived condition of Wrapping, not a state**: nothing new is
stored on the Conversation and the Lifecycle is untouched. It is read off the
wrap-up's settle facts plus whether a session is running, the same way
*blocked on you* is a badge on an active state rather than a state of its
own. The settling loop is the natural place to notice the narrowing, since it
already reads the settle facts on a cadence.

Settled decisions:

- The name is **Waiting on checks** — the codebase's word is checks, never
  CI, because GitHub checks cover more than CI.
- Drawn only while **nothing is running**: a fix session actively working a
  red check draws as plain Wrapping, never as idle waiting.
- The Notice is written **once per narrowing**. Leaving the condition — a fix
  session dispatched, a comment arriving and unsettling — and narrowing again
  later writes a fresh Notice.
- **No device push.** There is nothing for the human to do about it, so it is
  a Timeline line and a label only.

## Acceptance criteria

- [ ] A wrapping Conversation with review and comments settled, checks
      outstanding and no session running shows Waiting on checks on its card
      and in the sidebar, and its Timeline carries one Notice saying so.
- [ ] Dispatching a fix session, or a comment unsettling the wrap-up, returns
      the label to Wrapping; a later narrowing writes a fresh Notice rather
      than none or a duplicate of the first.
- [ ] No push notification is sent for the condition, and the settling rule's
      move to Done is unchanged.
