# 01. Foundations

## Goal

The agent type is real end to end, and Claude Code runs on the generalised
ground. Demonstrable: a Claude session launches with `--dangerously-skip-
permissions` passed by Verkstead, reads its skills from `/verkstead/skills`,
and behaves exactly as before; the store, the wire and the Profile form all
carry a per-type shape a later stage drops a backend into without touching
the schema again. The form still offers Claude alone — a type that cannot
launch yet would be a lie in a picker.

## Decisions in force

All from [ADR-0011](../../adr/0011-agent-backends.md); what bears on this
stage:

- **The skills move to `/verkstead/skills` for every backend.** A neutral
  path beside `/verkstead/bin`, which the sandbox already makes. The move
  takes the prompts in `crates/server/src/skills.rs`, the mount in
  `sandbox.rs`, and every `~/.claude/skills/...` cross-reference inside the
  bundled skills themselves — and the skills' Claude-specific wording is
  generalised as part of it, since stage 02's Guide tailoring is where
  per-backend advice will live instead.
- **Verkstead passes the bypass flags itself.** Claude's launch line gains
  `--dangerously-skip-permissions` here; the per-type argv mapping this
  stage shapes is where each later backend adds its own. Unattended is the
  product's promise rather than the account's configuration; the Sandbox is
  unchanged and stays the boundary.
- **A new-type Profile stores one home directory; Claude keeps its pair.**
  The store grows the per-type shape (the pattern for a new fact is a new
  table beside `profiles`, not a column migrated into it — see the
  `profile_models` precedent); the form takes per-type fields; validation
  and resolution follow the rules Claude's paths already follow.
- **`AgentType` stays a closed word list.** New variants arrive with their
  stages; an unknown word in the column is still a database from a newer
  Verkstead, refused by name rather than guessed past.

## Proposed tasks (provisional)

1. **Move the skills mount and every path that names it.** `/verkstead/
   skills` in the sandbox, the prompt constants, the skills' own
   cross-references, and the generalised wording sweep.
   - A Claude session reads the grilling skill from the new path; no bundled
     skill nor prompt says `~/.claude/skills` anywhere.
2. **Pass Claude's bypass flag from the launch line.** Extend the argv
   builder so flags are the backend's to say, and say Claude's.
   - The test suite's stub agents see the flag in the position the mapping
     defines; a real session runs unattended without the Profile's settings
     saying so.
3. **Per-type Profile storage.** The home-directory fact for new types,
   beside Claude's pair; reading, saving and the broken-Profile rules per
   type.
   - A Profile row of a new type round-trips through the store; a Claude row
     written before this stage reads back unchanged.
4. **Per-type Profile form and wire.** The type on the form, per-type path
   fields, `AgentType` on the wire growing its variants as stages land.
   - The form edits a Claude Profile exactly as today; the per-type shape is
     drawn from the discriminator rather than hard-coded.

## Re-verify at start

- The skills' cross-reference sweep: grep the bundled skills afresh for
  `~/.claude` — the skill set moves with the product and the list above will
  be stale.
- That `/verkstead/bin` is still how the executable reaches the sandbox
  (`Executable`, `sandbox.rs`) — the skills path is chosen to sit beside it.
- Whether the launch argv is still built in `Agents::argv`
  (`crates/server/src/sessions.rs`) with options appended after the prompt —
  the stub agents in the test suite read positions off that line.
- Whether anything beside the launch line has started depending on the
  Profile's own permission settings keeping runs unattended.
