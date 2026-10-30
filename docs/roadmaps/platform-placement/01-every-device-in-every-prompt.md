# 01. Every device in every prompt

## Goal

A session on any Conversation of a clustered Verkstead knows what machines
exist and what it stands on. Its prompt's devices section names every member
of the cluster — name, id, operating system, and whether this Conversation may
move there — and says which operating system this device runs; the Guide's
*Moving the work to another device* says the same and how to ask for a device
that is not permitted. The skills carry the two rules the grilling settled:
staging and breaking-down say when a line declares `on <os>`, and implementing,
next-task and addressing say that CI confirms and does not discover.
Demonstrable: a session on a Conversation with nothing ticked, on a cluster
with a Windows member, is told the member exists, that it may not move there
yet, and how to ask; a fix session for a Windows check reads that it is to
reproduce the failure there rather than fix it from the log.

## Decisions in force

- **Nothing ticked by default, and every device named anyway**
  ([ADR-0029](../../adr/0029-work-is-placed-by-platform.md), *Consent stays
  off by default, and is told*). The section is drawn on every clustered
  Conversation, permitted devices marked; the rule *nothing where the list is
  empty* goes.
- **This device's operating system at the head of the section**, in the
  platform's own word as the **Device Reading** spells it.
- **The ask route in this stage is the one that exists**: `verkstead ask`,
  with the Attach menu's *May be transferred to* row (ADR-0028). Stage 04
  rewrites the sentence when `verkstead transfer` becomes a request.
- **Declare only what can be verified nowhere else** (ADR-0029, *Declaring
  what the work wants*): a platform `cfg`, a platform's own component, a suite
  that runs only there. Prose declares nothing.
- **CI confirms; it does not discover** (ADR-0029, *What the skills say*). The
  addressing skill's *run the thing that failed* gains *where*: on the
  operating system the job ran on, moving where permitted and asking where
  not, never from the log alone.
- **No device inferred from a job name**: the agent decides.

## Proposed tasks (provisional)

1. **The section, for every member** — `transferable` lists every member with
   a permitted mark, headed by this device's operating system, drawn whenever
   the device is in a cluster.
   - A Conversation with nothing ticked names every member as not permitted.
   - A single-device Verkstead draws nothing, as today.
2. **The Guide's section** — *Moving the work to another device* says the list
   is every device, which are permitted, and how to ask for one that is not.
   - `verkstead guide` and the prompt agree on the words.
3. **Declaring, in staging and breaking-down** — the criterion beside the
   `on <os>` line in both skills, the task-list template showing the tail.
   - Both templates show a declared line and say when not to write one.
4. **Moving, in implementing, next-task and addressing** — the rule, with the
   addressing skill's failed-check step saying where to reproduce.
   - The addressing skill names the move before the fix.

## Re-verify at start

- The section is still `transferable` in `crates/server/src/skills.rs`, fed by
  `may_go_to` in `crates/server/src/transfers.rs`, applied at the launch point
  in `crates/server/src/sessions.rs` to the fresh prompt and the carried note
  alike.
- The Guide text is still `crates/cli/guide/core.md`, under *Moving the work to
  another device*.
- The member's OS word is still `store::Member::os`, read from the Device
  Reading; this device's is `crate::device::reading`.
- Stage 04 has or has not landed: where it has, the ask route to write is
  `verkstead transfer` as a request, not the Attach menu alone.
