# 04. Investigating as follow-up rounds configured

## What to build

Investigating rounds are stage 08's **Follow-up rounds** Activity configured,
not a new Activity: the skill `verkstead/investigating`, the evidence
`investigated`, the same Nothing-else mark, and an ending that checks nothing
was committed rather than reading the branch. The component already reads
`skill` and `evidence` options; this task wires the Investigate start path and
the Investigating detour (`runner::investigating`, `found_out`) to start it
with those options, and moves the investigating ending into it as an option
(where the Done signal over uncommitted scratch is accepted here and nowhere
else, ADR-0020 *Investigate* — an option of the evidence, not a rule of the
state).

A steered Investigating still finds its way back through
`investigations.rs` `homeward`, from the steer's recorded source, with the
scratch taken back out (`tidied`, `put_back`); that stays host-side.

## Acceptance criteria

- [ ] An Investigate Conversation runs its rounds under the Follow-up rounds component and lands Done over uncommitted scratch
- [ ] An investigation that committed anything ends as it does today
- [ ] A steered Investigating returns to the state it came from with the scratch taken back out, as `homeward` does
- [ ] The investigating tests pass unchanged, and no new `activity` block is declared
