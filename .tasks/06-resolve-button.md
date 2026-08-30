# 06. The resolve button

## What to build

Give a conflicted Done pull request one press that gets it resolved: a
**Resolve conflicts** button on its details pane, offered only while the
recorded fact says CONFLICTING — a button that resolves nothing would be
theatre, and the pane freshens the fact on open (task 04).

The press is a move of its own, not a steer. A steer into Wrapping
deliberately re-runs the review afresh; this press must not — the work was
reviewed and carried to Done, and a conflict is not a reason to read the
branch again. So the endpoint re-enters Wrapping with the review's settle
fact left standing, forgets the spent fix and conflict goes (the human has
read the record and asked for another round, the same forgetting Resume
does), and starts the wrap-up's watchers as found — which find the review
settled and run nothing for it, find the PR conflicted, and dispatch the
resolution session by task 02's rules.

What lands on the Timeline is the steer's shape: an Event saying the human
pressed this, with the machine's Moved line under it — somebody decided
this, and a long Timeline should say who. No device push: they were there
when they pressed it.

Once the resolution pushes and the checks on it go green, the settle rule
carries the Conversation back to Done on its own — with no review session
having run anywhere in between.

## Acceptance criteria

- [ ] The button appears on a Done PR's details pane only while the recorded
      fact says CONFLICTING, and the press lands a human-decided Event with
      the Moved line under it.
- [ ] The Conversation re-enters Wrapping with the review settle standing:
      no review session runs, and the conflict session dispatches with fresh
      goes.
- [ ] After a clean resolution and green checks the Conversation settles back
      to Done by the ordinary rule.
