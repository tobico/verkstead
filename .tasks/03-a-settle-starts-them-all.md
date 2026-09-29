# 03. A settle starts every ready stage

## What to build

A stage settling starts **every** ready stage of its roadmap rather than the
lowest one, up to **three stages of one roadmap at once** — a constant here, made
a setting by task 04. Where there are more ready stages than places, the
lowest-numbered ones start.

**A place is taken by every stage of the roadmap the record has in flight**,
which is the human's choice and includes a stage blocked on a Question Set and
one waiting to join the chain. Nothing new has to be stored for it: the record
already says which stages are in flight.

**Each start is its own act, and one that halts halts only itself.** A branch
already taken, a branch standing in the way, no git author configured, a fetch
git would not make, a companion that cannot be delivered — each of them says so
on the settled Conversation's Timeline as it does today, and the rest of the
ready stages are started anyway. This is where the carry-on stops reading *branch
already taken* as *nothing was started*.

**One notice per stage started**, as today, and one for a ready stage that waited
for a place, so a roadmap that has gone quiet says why. Every stage started
inherits its Pairings and its companions from the Conversation whose settle
started it, which is unchanged.

A halt before the join holds up only the stages that stand on the halted one:
they are not ready because it has not settled, its siblings' dependents are
unaffected, and it keeps its place until it settles or is closed.

## Acceptance criteria

- [ ] Two roots of one roadmap start together off one settle, each with a
      Conversation, a branch and a worktree of its own, and the Timeline names
      each of them.
- [ ] A fourth ready stage is not started while three are in flight, and the
      Timeline says it is waiting for a place.
- [ ] A halt on one — its branch already taken — leaves its siblings started; the
      halted stage's dependents do not start while a sibling's do, and resuming
      it lets its dependents start when it settles.
