# 04. The scheduler

## Goal

A declared roadmap runs by its declarations: **every stage whose dependencies
have settled starts**, up to three of one roadmap at once, whenever a stage
settles or the human presses Continue a roadmap. A stage that halts holds up
only the stages that stand on it. A roadmap that declares nothing runs in
order, as it always has. Demonstrable on a roadmap shaped like this one: 01 and
02 start together, 03 starts when 02 settles whatever 01 is doing, and 04 waits
for both.

## Decisions in force

- **[ADR-0021](../../adr/0021-parallel-stages.md), *What starts, and how
  many***.
- **Ready means every dependency settled**, by stage 02's record. Not merged.
- **An undeclared roadmap runs strictly in order**: each stage standing on the
  one before it is what *in order* means to the scheduler, so there is one
  scheduler rather than two.
- **Three stages of one roadmap at once, a server setting.** A stage waiting
  on the human or waiting to join **takes a place** — the human's choice.
- **Which ready stages, where there are more than places**: lowest number
  first. The roadmap's order is still the roadmap's own.
- **A halt before the join holds only dependents.** A usage limit, a failed
  start, a question unanswered: the rest carry on, and the halted stage still
  holds its place. **A halt after it has joined holds up every later join** —
  stage 03's chain is built one stage at a time and nothing joins until the one
  below has settled — which is not this stage's to fix and is what a stage
  waiting to join is waiting for.
- **Continue a roadmap starts every ready stage**, up to the limit. The card
  that named *the Stage that would be started* names them all.
- **A running roadmap that declares badly starts nothing and says why on the
  Timeline** — the other half of stage 01's refusal.
- **Declarations are read afresh at every start, off the top of the chain**,
  so a committed hand edit takes effect.
- **The guards that assumed one stage go**: the adoption refusing a roadmap
  with a stage in flight, `next_stage` returning one stage, and the carry-on
  reading *branch already taken* as *nothing was started*. What they protected
  against — a second Conversation on a stage already under way — is still
  refused, by the record.
- **Every stage's plan still stops on its breakdown Set.** Three stages
  starting is three Sets for the human; that was put to them and accepted.

## Proposed tasks (provisional)

1. **What is ready** — from the declarations and the record, the stages that
   may start now. AC: a stage with a dependency in flight is not ready; an
   undeclared roadmap has one ready stage at most; a finished roadmap has none
   and is complete only when nothing is in flight.
2. **A settle starts all of them** — the carry-on starts every ready stage up
   to the roadmap's limit. AC: two roots start together; a fourth ready stage
   waits for a place; the Timeline Event names each stage started.
3. **The roadmap's limit** — a setting, three unless changed, on the Settings
   page. AC: one runs a declared roadmap in order; a stage blocked on the human
   holds its place.
4. **A halt in the middle** — AC: a halted stage's dependents do not start; its
   siblings' dependents do; resuming it lets its dependents start when it
   settles.
5. **Continue a roadmap** — AC: the card names every stage the press would
   start; the press starts them; a roadmap with stages in flight can be
   continued for the ones that are ready.
6. **A bad roadmap, running** — AC: nothing starts, and the Event gives the
   reason stage 01's judgement gives.
7. **The words** — `CONTEXT.md`'s **Stage** and **Adopt**, and the `next-stage`
   skill's *no other plan in flight to check for*.

## Re-verify at start

- Stages 01 and 03 landed — and 02 through 03. Without the join, stages
  started side by side have no safe finish, and this stage must not land.
- **Check this roadmap's own lines still declare.** The stages before this one
  were planned on a server whose `next-stage` wrote its annotation over the tail
  of a line, so a declaration may have gone with it. Restore any that did before
  scheduling by them, or this roadmap is the mixed one this stage starts nothing
  from.
- `carry_on` in `continuing.rs` is still called from the settle loop in
  `settling.rs`, once, for the Conversation that settled.
- The global checkout lock in `lib.rs` still serialises only make-then-record,
  so several starts queue on it briefly rather than holding each other's
  fetches.
- A stage inherits its predecessor's Pairings and companions *from its
  predecessor*. With several predecessors, or none that is a Conversation,
  decide whose — the roadmap's own Conversation is the candidate — and put it
  to the human in the breakdown Set.
- Several sessions under one Implementation Pairing share one account's usage
  window; `limits.rs` stops a run on exhaustion. Check that three stopping at
  once reads as one thing to the human rather than three.
- Stage 05 adds the server-wide limit on top of this start. Leave the place
  where a start is permitted a single one.
