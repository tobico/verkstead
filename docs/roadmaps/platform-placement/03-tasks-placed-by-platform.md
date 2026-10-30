# 03. Tasks placed by platform

## Goal

A task that wants an operating system runs on it. A line in `.tasks/TODO.md`
carries `on linux`, `on macos` or `on windows` after its link, read by the one
checklist parser; before launching a task the driver moves the Conversation to
the first reachable ticked device of that platform where it is not already on
one, and launches the task's session there. An inline run whose proposal
wants exactly one platform starts there the same way. Where no ticked device
of the platform is reachable, this stage writes a Notice and runs the task
where it is — stage 04 replaces that Notice with a question. Demonstrable: a
backlog with one Windows task, worked on Linux with a Windows device ticked,
moves for that task, works it there, and the next undeclared task runs where
the work now is.

## Decisions in force

- **A task line declares as a stage line does**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Declaring what
  the work wants*), through the parser both files share. A backlog that
  declares on some lines and not others is still a backlog: a bare task line
  wants no platform, because a task list has no dependencies to confuse it
  with.
- **The platform is a preference**; the Notice this stage writes is the
  fallback stage 04 turns into a Set.
- **Verkstead moves declared work; the agent never calls transfer for it**
  (ADR-0029, *Placing declared work*). The move is the ordinary Transfer with
  nothing running, which runs where it stands.
- **The first reachable ticked device of that platform**; two ticked and one
  asleep means the awake one.
- **The work stays where it is afterwards**: an undeclared task, the finish and
  the wrap-up run wherever the last move left the work.
- **An inline run wanting one platform starts there**; wanting several, it
  starts where it is and the agent moves.

## Proposed tasks (provisional)

1. **Reading a task's platform** — the checklist entry's tail yields a platform
   for a backlog line as it does for a stage line.
   - `- [ ] 03: Title — [details](03-x.md) — on windows` reads `windows`; a
     bare line reads none.
2. **The placement before launch** — the driver, choosing the next task, asks
   where it wants to be, picks the device, moves, and launches there.
   - The Timeline says the task was moved for and to which device.
3. **The fallback Notice** — none ticked, none reachable, or none of that
   platform linked: a Notice naming the task and the platform, and the task
   runs here.
4. **The inline run** — a proposal wanting one platform places the
   implementation session.
5. **The breaking-down skill** — the template's declared line and when to
   write one, if stage 01 has not already put it there.

## Re-verify at start

- `crates/server/src/checklist.rs` is still the one parser and `Entry::after`
  the tail; stage declarations are still read in `crates/server/src/stages.rs`
  around the platform word.
- The driver still picks the next task in `next_step` in
  `crates/server/src/runner.rs` and launches it from there; the inline run
  follows the handoff in the same file.
- A Transfer with nothing running still runs where it stands, from
  `crates/server/src/transfers.rs`, and the preflight is still
  `crates/server/src/preflight.rs`.
- Stage 02 landed, for `wants` on the record of the inline run's proposal.
