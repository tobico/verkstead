# 01. End a propose-then-fix session on quiet with nothing pending

## What to build

The wrap-up's review session and the batch comment sessions are the one kind of
session Verkstead waits to see exit by itself — but every session is an
interactive claude, which idles when its work is done and never exits. So after
such a session lands its fixes it sits idle forever, the review never settles,
and a roadmap's next stage never starts until somebody quits the session by
hand through the Screen view.

End these sessions the way the others are ended, on a rule of their own: **quiet
for a dedicated grace period, with no unanswered blocking Question Set on the
Conversation**. A session idling on a blocking ask is working, however long the
human takes, and is never ended; an unanswered *deferred* Set holds nothing
open — its answers reach a later session by design. When Verkstead ends a
session under this rule, that is the session finishing cleanly: the runner
treats it exactly as it treats one that exited well, so the review settles, the
wrap-up can reach Done, and the next stage starts.

Mirror the existing landed-plus-quiet endings: race the session's own exit
against the new condition, and end the sandbox when the condition wins. The
grace is its own pacing value, defaulting to 60 seconds — deliberately longer
than the 5-second grace fix sessions use, because ending early here reads as a
review that found nothing — and tunable by tests the way the existing grace is.
The store needs one new read: whether a Conversation has any blocking Set with
no Response yet.

## Acceptance criteria

- [ ] A propose-then-fix session that goes quiet with no unanswered blocking
      Set is ended after the grace, reads as a clean finish, and its review
      settles — so a wrap-up whose fixes have landed carries on to Done and the
      next stage with nobody touching a Screen.
- [ ] A session idling on an unanswered blocking ask is left alone
      indefinitely, and an unanswered deferred Set holds nothing open.
- [ ] Anything the session prints puts the whole grace back on the clock, so a
      session still talking is never cut off.
- [ ] The rule is proven both ways by runner tests using the stand-in agent, as
      the landed-plus-quiet rules are.
