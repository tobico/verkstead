# 01. The places on the server

## What to build

A second limit in front of the one place an unattended start is permitted: **four
Conversations across the whole server**, counted whatever roadmap or Process they
belong to, beside the roadmap's own three.

**What takes a place.** A Conversation with a **session running or a driver
registered**. Both registers, rather than the sessions one alone: a stage waiting
to join the chain has no session at all — the hold stands in front of the launch
— and a Conversation between two task sessions has none for a moment either, so
counting sessions would let a fifth in through both gaps. The drivers register is
what stays put across both, every driven run holding one for as long as its task
lives. So a running grilling takes a place, a stage idling on a Question Set
takes one, a stage waiting to join takes one — and a Conversation that is Done,
Draft or **stopped** takes none, a stop being raised as the driver lets go.

Which leaves the two limits deliberately asymmetric, and it is worth knowing
which way: a **halted** stage keeps its place under its own roadmap's limit, the
record having it in flight, and takes none under the server's, nothing being run
or driven. A server of halted stages goes on starting other roadmaps' stages
while each halted stage's own roadmap stands still.

**Both registers are facts about this process**, so a server that has just come
back holds no places at all until the startup resume takes its runs back up —
which is the reading the stall sweep already waits for, and the right one here
too.

**Where it goes.** The carry-on counts the places and hands the reading that
decides what starts the places left over, beside the roadmap's limit it already
hands it — passed in rather than read there, so that where a start is permitted
stays the one place and how many are permitted stays the caller's to say. A ready
stage that fits under its roadmap's limit and not under the server's is **held
with a sentence of its own**, saying it is waiting for a place on the server
rather than on its roadmap: a roadmap that has gone quiet with work left in it is
one nobody can tell from a roadmap the scheduler forgot.

**Nothing about the presses changes.** *Continue a roadmap*, Start and Resume
read their own way in and start whatever they were going to start, over the limit
and counted from then on. That is a test rather than a change, and it is the half
of this worth proving hardest.

Four unless changed, a constant beside the roadmap's three. The setting that
moves it is task 04.

## Acceptance criteria

- [ ] A running grilling takes a place, a stage idling on a Question Set takes
      one, and a stage waiting to join the chain takes one; a Conversation that
      is Done, Draft or stopped takes none.
- [ ] With four places taken, a settle whose roadmap has a ready stage under its
      own limit starts nothing, and the Timeline says that stage is waiting for a
      place on the server; with three taken it starts as it does today.
- [ ] A press at four still starts: *Continue a roadmap* starts every ready
      stage, and the Conversations it made are counted from then on.
