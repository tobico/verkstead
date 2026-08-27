# 05. End the follow-up on the mark, and land in the wrap-up

## What to build

The rule that ends a Follow-up, replacing task 03's interim stop. A
follow-up is done when three things hold together:

1. the **latest settled Set** of the follow-up carries the Nothing-else mark
   from task 04,
2. the session is **idle**, and
3. **no Set is open** on the Conversation — deferred asks never count, as
   everywhere else.

Then the session is ended and the Conversation **re-enters Wrapping** with
the wrap-up's settle facts as they stand, its watchers recomputed the way a
steer into Wrapping recomputes them. "Back to Done" is the wrap-up's own
settling rule and nothing else.

- **Unsettle checks at the landing where the follow-up recorded commits**,
  so the settling rule cannot fire Done before the checks watcher's first
  poll sees the new run. A pure Q&A follow-up that pushed nothing lands with
  everything settled and passes straight through to Done.
- **The latest Response decides.** A Set asked after an end-marked Response
  reopens the follow-up — the human may pick Nothing else and write "one
  more thing" in the comment, the agent does the thing and asks again, and
  that newer Set's own Response settles it afresh. The mark is never sticky.
- The idle-with-no-Set-and-no-mark case is **not** this task's: task 06's
  rescue covers it. Until 06 lands, such a session simply sits, which is
  today's behaviour for every state.
- **A dead session is a stop** — the responding rule: nobody is dispatched
  to finish it, the stop's Notice says what happened, and any Set it left
  open is closed as the stop is raised, so no question nobody is behind
  keeps the card blocked on you.

## Acceptance criteria

- [ ] An end-marked latest Response plus an idle session with no open Set
      ends the session and moves the Conversation to Wrapping; with new
      commits it narrows to Waiting on checks and reaches Done when the
      checks go green, and with no commits it reaches Done at once.
- [ ] A Set asked after an end-marked Response keeps the follow-up open, and
      its own Response decides.
- [ ] A follow-up session that dies stops the Conversation with a Notice and
      closes any Set it left open.
