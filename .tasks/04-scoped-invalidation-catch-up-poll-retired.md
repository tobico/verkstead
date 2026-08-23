# 04. Scoped invalidation, catch-up, poll retired

## What to build

Make the viewer react to a Nudge's scope, hand the poll's duties to catch-up,
and retire it.

Scoped reaction: one mapping table in the nudge module from kind to the query
keys it invalidates — conversation-scoped kinds invalidate only that
Conversation's queries (and the lists that show its badge/state), global kinds
their lists. The server never learns the viewer's cache layout; the table is
the client's. Unknown kinds keep falling back to invalidate-everything. The
practical wins to verify: while a session talks, `repos`, `profiles` and
above all the `gh`-backed pull-request query stay quiet — pull-request
refetches only on commit-flavoured movement of its own Conversation.

Catch-up, replacing the poll's backstop: invalidate everything on every SSE
(re)connect and on every return to visibility, with refetch-on-focus staying
on — events missed while suspended or disconnected are recovered wholesale.
Then delete the 10-second conversation poll.

Liveness rode that poll: the waiting/disconnected verdict cycled with the
refetch. Add the `liveness` kind, announced scoped to its Conversation when
the agent's long-poll connects and when it drops, so the badge still flips
with the poll gone.

## Acceptance criteria

- [ ] While a session talks with a Conversation open, only that
      Conversation's queries (plus affected lists) refetch; `repos`,
      `profiles` and pull-request are not re-read on transcript batches
- [ ] Pull-request re-reads on commit-flavoured Nudges of its Conversation,
      and otherwise stays quiet
- [ ] SSE (re)connect and visibility-return each invalidate everything;
      unknown kinds still do
- [ ] The 10-second poll is gone, and the liveness badge still updates via
      the new `liveness` kind
