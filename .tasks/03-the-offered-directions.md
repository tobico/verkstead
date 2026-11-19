# 03. The offered Directions

## What to build

A Process declares which Directions its interview offers (Set 1765, Q27); this
task builds the half of that the interview owns, ahead of stage 11 writing the
declaration. The interview takes the **offered Directions** as a start fact
(an option, defaulting to all three until a Process says otherwise), and the
host draws the proposal's chooser from it: the Set's proposal carries the
offered list, the viewer draws those and no others, and **one Direction is no
chooser at all** — the proposal is accepted or sent back with that one
Direction.

That reaches through the schema (the proposal and Response's `direction`, and
`validate.rs`, which today assume the same three every time), the viewer's Set
sheet (`web/src/directions.ts`, `web/src/set/`), and `verkstead done`, which
refuses a Direction that was not offered. The Guide and the grilling skill say
what the session sees. Copy follows `docs/copy.md`.

## Acceptance criteria

- [ ] An interview started offering a subset of the Directions puts a proposal whose chooser draws only those, in the viewer and in the stored Set
- [ ] An interview offering one Direction shows no chooser, and accepting the proposal picks that Direction
- [ ] A Response or a `done` naming a Direction not offered is refused, saying which were offered
- [ ] With no option given, every proposal offers all three as today and the suite passes unchanged
