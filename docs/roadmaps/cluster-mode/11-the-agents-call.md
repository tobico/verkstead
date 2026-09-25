# 11. The agent's call

## Goal

The agent moves the work itself. While drafting, under *May be transferred
to* in the device select's panel, a tick per other device — the drafting
device always permitted — and the same ticks in the Transfer dialog after.
Where the list is not empty the session's prompt names those devices with
their OS and says when to reach for `verkstead transfer <device>`; the Guide
carries a section. The call runs the preflight and exits non-zero naming what
is missing, refuses a device off the list, and otherwise records the request
and returns; the driver moves the Conversation when the turn ends and the
Timeline says *Transferred to B at the session's request*. Demonstrable end to
end: an agent on Linux asked to fix a Windows build moves itself to the
Windows VM and carries on there.

## Decisions in force

- **The ticks are the consent**; no confirmation follows
  ([ADR-0020](../../adr/0020-cluster-mode.md), *The agent's call*). A press
  on the Timeline was rejected.
- **The call follows `verkstead done`**: a request the driver acts on at the
  turn's end, reached through the Conversation-scoped base URL, so nothing
  names the Conversation but where it was asked from.
- **Preflight before it returns**, so the agent can choose another device or
  ask the human; a Notice-only failure was rejected.
- **Name or id**, the prompt listing both; an ambiguous name refused naming
  both.
- **Said in the prompt only where the list is not empty**, and in the Guide.
- **A human-pressed transfer ignores the list.**

## Proposed tasks (provisional)

1. **Permitted devices on the record** — a table per Conversation, the ticks
   in the select's panel and in the Transfer dialog, the drafting device
   implicit.
   - A device that leaves the cluster drops off the ticks.
2. **The route and the CLI verb** — `transfer` on the session API, the CLI
   arm with `--server`, the preflight run synchronously, the request recorded.
   - Off-list, unreachable, or unmatched Repo each exit non-zero with the
     reason.
3. **The driver's move** — the pending transfer honoured at the turn's end,
   the Timeline wording, stage 09's move run.
   - The agent's last words on A precede *Transferred to B at the session's
     request*.
4. **The prompt and the Guide** — the permitted list threaded through every
   prompt builder, the Guide section.
   - A Conversation with no ticks says nothing about transfer.

## Re-verify at start

- Stage 09 landed; stage 10 may or may not have — the move works either way.
- The CLI's commands are still in `crates/cli/src/lib.rs` with the client in
  `crates/cli/src/client.rs`; the done request is still
  `crates/server/src/done.rs` acted on by the driver.
- Prompt builders are still in `crates/server/src/skills.rs` with
  `alongside`, `attached`, `naming` and `folded` applied at the one launch
  point.
- The Guide text lives in the CLI and is what `verkstead guide` prints.
