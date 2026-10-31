# 02. The move before the first session

## What to build

A stage whose roadmap line declares a platform (`on windows`) is moved to a
device of that platform before its first session — the planning session —
starts. Made at home as every stage is, so home holds the birth key and the
tombstone and stage 05's driving holds (ADR-0029, *Placing declared work*).

- **The stage carries its platform.** The `Stage` a start is made from gains the
  platform its line declares, read in the same reading of the roadmap off the top
  of the chain as its dependencies and its brief (`stages::next_stage` /
  `startable`, through `declarations::read`) — never cached, never off the
  settling stage's Worktree. A roadmap that declares nothing has no platform on
  any stage.
- **Placed by the existing placement module** (`placing.rs`, which stage 03/04
  built for tasks and the inline run): a new kind of work, a stage, by its label —
  its `Work`, the store's `PlacedWork` and the Transfer's `AskedBy` each gain a
  stage case, so the stage's Timeline reads *Transferring to nuc-win* for stage
  06, which wants Windows, as it does for a task. The device is the first
  permitted device of that platform whose preflight holds nothing, read through
  `platform::os_mark` (a WSL counting as Linux) — the rule a task is placed by.
- **When.** In a settle's start (`continuing::start`), after the stage is
  recorded (`store::start_stage`) and before the Notices are written, so the
  roadmap's Notice can name where it went. Where it is moving, the planning
  session is not launched here: the far end's arrival Resume plans it (the
  `backlog_again` → `stage_to_plan` → `plan_stage` path), and this device's
  driving registration is let go.
- **Once, where it starts, like an inline run** (Set 1549, Q2). A Resume of the
  stage never places it again: a stage moved home by hand stays home, and its
  declared tasks are still placed on their own by the task rule. So the platform
  need not be stored on the stage's record.
- **The roadmap's Notice names the device**: the stage started as a conversation
  of its own on its branch, and is moving to nuc-win. Words by `docs/copy.md`.
- A stage wanting this device's own platform starts here, untouched, with
  nothing extra said.

This task covers the move. What happens where no device can take the stage
(the Set, the Notices, *Run it here*) is task 03; the *Continue a roadmap* press
is task 04.

## Acceptance criteria

- [ ] The stage a settle starts carries the platform its line declares, read at
      the top of the chain; a hand edit adding `on windows` to a waiting stage's
      line before it starts is what that stage is placed by.
- [ ] A declared stage with a ticked, awake, clean device of its platform is
      moved there before any session runs at home; its Timeline shows the
      Transfer asked on Verkstead's account for the stage, and the roadmap's
      start Notice names the device.
- [ ] A stage wanting this device's platform starts its planning here exactly
      as before.
- [ ] End to end over two devices in the suite: the Windows stage's planning
      session runs on the far device, it reports home, and when it settles home
      starts the stage that stands on it (stage 05's driving).
