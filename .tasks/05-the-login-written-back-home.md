# 05. The login written back home

## What to build

As a session away from home ends, **what it did to the login goes back to the
home device** over the same peer route, and that device writes it into the
account the way a local session's ending writes one. A login is the one thing in
a Built Root a session genuinely changes — the harness refreshes an OAuth pair
as it works — so an account lent out and never written back would be an account
signing itself out a session at a time.

**Last write wins.** Two machines refreshing one login at once may sign one of
them out; that is accepted rather than locked against, and lending a Profile out
exclusively was rejected as a lock nobody asked for. Nothing is merged and
nothing is held: the write that arrives later is the one the account keeps. A
sign-out that results reads as the Profile being broken there, and the fix is a
login on its home device — which is task 07's business.

**Only what changed travels.** A login the session replaced rather than wrote
through goes back whole; one that is still exactly what came down is not written
at all; and a login made where the account had none is handed over — the same
three cases a local session's ending already tells apart.

**A home that has gone away by then is said rather than swallowed.** The mirror
is left where it is, and the Timeline carries a sentence saying the account was
not written — the next session at home may find itself signed out, and that is
worth a line rather than a silent loss. Nothing retries in a loop.

## Acceptance criteria

- [ ] A token refreshed inside a session on B is in A's account once that
      session ends.
- [ ] A login the session replaced goes back whole; one it left untouched is not
      written; one made where the account had none is handed over.
- [ ] Two devices refreshing one login end on the later write, with nothing
      locked and nothing merged.
- [ ] A home unreachable at the end leaves the mirror in place and puts a
      sentence on the Timeline saying the account was not written.
