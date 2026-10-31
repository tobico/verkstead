# 02. What the work wants, on the chooser

## Goal

The grilling's closing Set says which platforms the work wants, and the human
ticks devices at the moment they pick a direction. The `proposal` block gains
`wants`, a list of the three platform words each with a reason; the chooser
draws the *May be transferred to* ticks under the three directions, devices of
a wanted operating system first with the reason beside them, and a line saying
so where the cluster has no device of that platform. The grilling skill tells
the agent to write it. Demonstrable: a closing Set wanting Windows, answered
on a phone, ticks the Windows device and picks a roadmap in one submit, and
the record shows both.

## Decisions in force

- **`wants` on the proposal, a platform and a reason each**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Declaring what
  the work wants*). A bare list of words was turned down: the reason is what
  the human reads beside the tick. Leaving the proposal alone and asking with
  an ordinary Question was turned down: the chooser is where they are already
  deciding.
- **The three words and no others**, refused by the schema's validation as a
  stage line's platform is refused.
- **The ticks are the same ticks** — the Attach menu's rows of ADR-0028,
  drawn in the open under the chooser rather than behind a menu, recorded on
  the Set and applied at Submit exactly as an addition is.
- **A cluster with no device of the wanted platform says so** on the chooser
  and offers nothing to tick; the work will run where it is.
- **A Verkstead in no cluster** draws `wants` as a line of prose and no ticks.

## Proposed tasks (provisional)

1. **The schema** — `wants` on `Proposal`, validated to the three words with a
   non-empty reason; the TypeScript types regenerated.
   - A fourth word or an empty reason refuses the Set naming the field.
2. **The chooser** — the ticks under the directions, wanted platforms first
   with the reason, the no-device line, held on the Set and applied at Submit
   as an addition is.
   - Submitting a pick and a tick together records both; the Response's
     `added` names the device.
3. **The grilling skill and the Guide** — the closing Set's template shows
   `wants`, and says when to write one.
   - A proposal with no platform wants nothing and draws as today.

## Re-verify at start

- `Proposal` is still `crates/schema/src/set.rs` with `check_proposal` in
  `crates/schema/src/validate.rs`; `Direction` is exported to `types.ts`.
- The chooser is still injected in `web/src/set/Answering.tsx` from
  `web/src/directions.ts`; the Attach menu's device rows are the same file's
  *May be transferred to* section, applied by
  `crates/server/src/answer_additions.rs`.
- The grilling skill's closing-Set section is `crates/server/skills/grilling/SKILL.md`.
- Stage 01 has or has not landed; the words for *permitted* should match it.
