# 06. Stages placed by platform

## Goal

A stage line's `on windows` places the stage. A roadmap's stages inherit the
roadmap Conversation's *May be transferred to* ticks; a declared stage is made
on its home device as every stage is and moved by the ordinary Transfer to the
first reachable ticked device of its platform before its first session starts;
where none is reachable or none is ticked, stage 04's Set asks, and a roadmap
with no device of the platform linked says so once and runs the stage at home.
Demonstrable end to end: a roadmap with a Windows stage, written on Linux with
the Windows device ticked at the closing Set, starts that stage on the Windows
device unasked, and the Linux device starts the stage that stands on it when it
settles.

## Decisions in force

- **Made at home and moved**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Placing declared
  work*), so the birth key and the tombstone are home's and stage 05's driving
  holds. Making the stage on the far device through the relay, as the composer
  makes a draft, was not taken: home would hold no row.
- **The ticks are inherited** through the one funnel that carries Pairings and
  companions.
- **The first reachable ticked device of that platform**; otherwise stage 04's
  Set, with the stage waiting on it taking a place as waiting on the human
  does.
- **The platform is a preference**: *Run it here* starts the stage at home.
- **The roadmap's declaration is read as it always was**, off the top of the
  chain at every start; ADR-0021's *acted on by nothing* is what this ends.

## Proposed tasks (provisional)

1. **Inherited ticks** — the permitted devices copied in `continuing::settle`
   beside the Pairings; the Transfer dialog of a stage shows them.
2. **The move before the first session** — a declared stage, once made, is
   moved to the chosen device and its first session starts there; the Notice
   that says which stages started names the device.
3. **The gate for stages** — stage 04's Set for the askable cases; a Notice
   and a start at home for a platform with no device.
4. **The roadmap pane** — a stage's platform, shown today, gains the device it
   was placed on.
5. **The glossary** — **Stage**'s platform sentence and **Permitted Device**
   rewritten to what is now true.

## Re-verify at start

- Stages 04 and 05 landed: the Set this fires and the driving that makes a
  moved stage safe.
- The inheritance funnel is still `settle` in `crates/server/src/continuing.rs`;
  the ticks are still `crates/store/src/permitted.rs`.
- A stage is still made from `startable`'s answer in
  `crates/server/src/conversations.rs` and `crates/server/src/stages.rs`, and
  the platform word is still parsed beside the dependencies there.
- The Transfer with nothing running is still the human-pressed path in
  `crates/server/src/transfers.rs`, and the preflight's `Lacking` still names
  what a device lacks.
