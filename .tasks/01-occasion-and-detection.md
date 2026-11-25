# 01. The occasion and the launch-time carry-on

## What to build

The tracer bullet: the machinery that carries a mid-turn cut-off on, proven
end-to-end on the simplest case — a **force-stopped grilling** (a grilling
cut-off is always mid-turn, so no state-reading is needed to decide it yet).

Add one new **occasion** to `carrying::Occasion` for a stop cut-off (alongside
`Arrived`, `Widened`, `Kept`), and the column on `continued_sessions` that
records it — through `ALTER TABLE` the way `widened_by` and `kept_here_by` were
added, so an older database takes the same path. Give it its clause in
`carrying::Occasion::opening`, its arm in `carrying::carried_on` (the "the session
that stopped here has been resumed and carries it on" wording — it does **not**
restate why it stopped, the stop Notice already did), and its arms wherever
`Instead` is worded for the fallback.

Add the detection in `resume::recompute`, before it spawns the driver: where there
is **no** `continued_sessions` row yet (`store::carrying_a_conversation` is false)
and the newest session (`store::the_last_session`) was cut off mid-turn, write the
row so `sessions::continuing` resumes it. Guarding on "no row yet" keeps this clear
of the Arrived / Widened / Kept paths, which pre-write their own row and also reach
`recompute`. Where `the_last_session` is `None` (no session id Verkstead knows),
no row is written and the launch stays Verkstead's own Resume, said with the
`Instead::NoSession` clause.

Prime the **note** only where the cut-off session was idling on a Question Set:
point it back at the Set and its `verkstead answers` line (the blocking `verkstead
ask` died with the process), with the ids unchanged — this is the same device, so
no renumbering. Otherwise no note: `--resume` alone. The Sets it was idling on
stay **open** (held, not locked), which the existing `continuing` path already does
once a row is present.

## Acceptance criteria

- [ ] A force-stopped grilling, on Resume, is carried on with the harness's own resume, and its Timeline Notice says so.
- [ ] The Question Sets it was idling on stay open for the resumed session, and the primed note points it at them and the `verkstead answers` line.
- [ ] A cut-off whose session id Verkstead does not know stays Verkstead's own Resume and its Notice says why (`Instead::NoSession`).
- [ ] The new occasion does not disturb the Arrived / Widened / Kept paths, which pre-write their row.
