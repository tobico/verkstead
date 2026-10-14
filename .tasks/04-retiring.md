# 04. Retiring

## What to build

Taking a memo back, from a session and from the Timeline, and what the record
and the prompt do when one is.

**From a session.** `verkstead memo --retire <id> [reason]`, the id being the
memo's Event id as the prompt names it, the reason optional and taken as the
rest of the line. It hits a route of its own beside the one task 02 added,
under the same building-session rule and the same refusals — plus two of this
command's own, each refused by name: a memo that is not this Conversation's,
and one already retired. Accepted, the side table row gets `retired_at`,
`retired_by` in the words of the kind that retired it (*the session that
worked task 05*) and the reason. The CLI confirms on stderr and prints nothing
on stdout.

**From the Timeline.** A Retire press on the card, under the viewer's own
key-gated namespace, with `retired_by` reading *you* and no reason. A retired
card offers Restore in its place, which clears the three columns; because
Restore is there, Retire asks nothing first. Both are one press and the card
redraws from the server's answer.

**What follows.** A retired memo leaves the prompt — task 03's reader skips
it — and stays on the Timeline drawn the way task 01 draws a retired one:
dimmed, with who retired it and the reason. The Event is never deleted and no
second Event is written: retirement is a change to the memo's own row, and the
record keeps one card per memo.

## Acceptance criteria

- [ ] A session's `verkstead memo --retire 42 fixed in 8f3c2a1` marks the memo
      retired with its stamp and reason, and the next session's prompt no
      longer carries it; the one after that carries the rest.
- [ ] Retiring a memo of another Conversation, or one already retired, is
      refused by name with a non-zero exit.
- [ ] The Retire press on the card marks it retired by *you* without a
      confirmation, and Restore brings it back into the prompt.
- [ ] A retired memo's card reads who retired it and why.
