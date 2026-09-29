# 04. The way back

## What to build

The rule runs both ways. A Conversation closed to make way — or closed by hand
long before — can be steered back into work, and the pull request it is on may
by then be another Conversation's. See
[ADR-0020](../docs/adr/0020-a-conversation-has-a-process.md), *One open
Conversation per pull request*.

Today such a steer makes its checkout again from the branch and, where the
branch is standing in the other Conversation's worktree, answers *Git would not
make the worktree* with the reason in the server's log. That is the one refusal
described as having nothing for the human to correct, and here there is
something.

**A Steer into a state something runs in**, by a Conversation with a pull
request on its record, asks who else is on that pull request before it plans its
checkout:

| The other Conversation is | The steer |
|---|---|
| Done | closes it, by the ordinary Close, then makes the checkout |
| Closed, Archived | goes ahead |
| still at work | refused, naming it and leading there |

A steer into Done runs nothing and asks nothing.

**Uncommitted changes are asked about first**, as task 02 asks: the steer's
submit stops naming the Conversation that would lose something, and a second
submit goes ahead. The pending steer stands while it is asked.

**The Resolve conflicts press** is pressed on a Done Conversation, and a Done
Conversation is never beside another open one on the same pull request, so it
needs no rule of its own. Where it finds the branch standing somewhere else
anyway — a checkout that is nobody's — it keeps the refusal it has.

**Stacks are not walked here.** A steer is about the Conversation's own pull
request; the neighbours are a Fix Merge Issues start's to clear.

## Acceptance criteria

- [ ] A Closed Conversation steered into Wrapping or Follow-up, whose pull
      request a Done Conversation took, closes that one and has its checkout
      made from the branch
- [ ] The same steer where the other Conversation is still at work is refused
      naming it, and nothing is closed or made
- [ ] Where the other holds uncommitted changes the submit stops naming it, the
      pending steer still standing, and the second submit goes ahead
- [ ] A steer into Done closes nothing
- [ ] A steer by a Conversation nobody else shares a pull request with behaves
      as it does today
- [ ] The Steer form draws the new refusal with its link, and the Timeline of
      the steered Conversation names the one it closed
