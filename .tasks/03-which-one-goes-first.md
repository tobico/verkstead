# 03. Which one goes first

## What to build

The order the free places are handed out in, where more than one roadmap is
waiting for one: **oldest roadmap first, and within a roadmap lowest number
first.**

Within a roadmap the order is already the roadmap's own — the ready stages come
back lowest-numbered first and a place is spent by a stage that starts and by
nothing else — so what this adds is the choice *between* roadmaps, and the rule
that the within-roadmap order survives it.

**Oldest is the roadmap's own age**, read off the Conversation that wrote it,
rather than how long any stage of it has been waiting. Nothing is stored for
this, which is what keeps it from going stale: a timestamp saying when a stage
was first held would be a second record beside a readiness that is worked out
afresh every time, and the two would disagree the moment a dependency settled
under it.

**A roadmap already at its own limit is passed over**, however long it has been
waiting and however many places the server has free: the two limits are both in
force and the roadmap's is the stricter one here. The place goes to the next
roadmap that can use it rather than standing empty.

**And the places run out mid-list.** With one place free and three roadmaps
waiting, one stage starts and the other two are told they are still waiting, in
the same breath and on their own roadmaps' Timelines — a roadmap passed over in
silence reads as a roadmap forgotten.

## Acceptance criteria

- [ ] Two roadmaps each holding a ready stage with one place free: the older
      roadmap's stage starts and the younger one's waits.
- [ ] Within one roadmap the lowest-numbered ready stage takes the place, whatever
      order the others were held in.
- [ ] A roadmap already at its own limit of three is passed over with a server
      place free, and the place goes to the next roadmap waiting.
