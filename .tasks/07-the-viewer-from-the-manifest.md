# 07. The viewer from the manifest

## What to build

Words come from the manifest (Set 1765, Q29). The viewer stops keeping its own
tables of Processes and states and draws from what the server sends:

- the picker's rows and descriptions, for the five built-ins as for any other;
- each state's word on the Conversation card, the Timeline and the sidebar;
- the Target field, drawn where the Process's manifest says it takes one;
- an unknown state word — a plugin removed under a Conversation — drawn as the
  word with its trouble.

`web/src/workbench/states.ts`, `processes.ts`, the mirrored `LANDED`, and the
server's `takes_a_target` and `narrows_the_wrap_up` lists go. Spelling rules
in `docs/copy.md` hold for the built-ins' words; a third-party word is the
plugin's own.

**The Nothing-else box and the proposal chooser follow the running Activity,
not the state word**: a Set from a Follow-up rounds session carries the
Nothing-else box, and a Set from an Interview honours a `proposal`, whatever
the state is called. Today both are gated on the state in the store
(`Directing::NotGrilling`, and the `Ending` on follow-up and investigating
rounds) — those gates read which Activity launched the session instead.

## Acceptance criteria

- [ ] The hand-kept tables named above are gone from the tree
- [ ] A third-party Process installed appears on the picker with its words, and
      a Conversation on it draws its state words on the card
- [ ] A fixture Process whose Follow-up rounds state is called something else
      still gets the Nothing-else box, and one whose Interview state is called
      something else still gets the proposal chooser
