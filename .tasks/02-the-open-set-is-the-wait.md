# 02. The open Set is the wait

## What to build

While Verkstead's own placement Set (task 01) is open, the work is waiting on the
human exactly as a session idling on an ask is — and nothing about Verkstead may
read it as stalled, free or forgotten.

- **It takes a place.** Places on the server are counted off two registers, the
  drivers and the running sessions (`Drivers::taking`), and while the gate waits
  there is neither: no session has been launched. Hold a driver registration for
  the Conversation for as long as the Set is open, let go when it settles, so a
  Stage waiting on the Set takes a place as waiting on the human does
  (brief, *Decisions in force*), and a place comes free when it settles.
- **The stall sweep leaves it alone**: a Conversation in Implementing with an
  open Verkstead Set is waiting, not undriven, and is never stopped as *nothing
  is driving it*.
- **A restart re-finds the open Set.** The startup resume reaches the same
  launch again; placement must find the open Set of its own for this Conversation
  and wait on it again, never put a second one. Likewise any other path that
  reaches the same launch while the Set is open (a Resume press, a second
  driver) waits on the open Set.
- **An answer that lands while nothing is running** — after a restart, say —
  still runs its Option.

## Acceptance criteria

- [ ] With a placement Set open, the Conversation counts in the places taken, and
      settling the Set frees the place (a held stage starts when it is the last
      place).
- [ ] A restart with the Set open puts no second Set; answering it afterwards
      still moves the work or starts it here.
- [ ] The stall sweep leaves a Conversation waiting on its placement Set running.
