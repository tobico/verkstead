# 01. Manual Task on the Timeline

## What to build

The Event kind a Manual Task is recorded as, end to end: the store holds it,
the wire carries it, and the Timeline draws it.

A Manual Task's instruction is one markdown string — the free text the human
typed — so it rides in the Event's body beside the Brief and the handoff, with
no side table and nothing joined in beside it. Note that the Timeline read is
already at sqlx's column limit; if a payload table ever becomes necessary, the
fetch-and-merge pattern documented there is the way, but nothing here needs one.

Nothing writes one yet but the tests. This slice is the record: an instruction
put on a Conversation's Timeline comes back off it whole, and is drawn as its
own row rather than as an unknown kind.

The two words this feature adds also land here, because everything after this
uses them. **Manual Task** is the human's own choice of name, over "Errand" and
"Manual Step", and has to stay distinct from the **Step** (the unattended unit
with a done-file signal), from the `.tasks/` backlog's tasks, from the **Hold**,
and from the *Take over manually* Remedy — it is none of them. **Stalled** is
the condition tasks 04 onwards detect, and has to stay distinct from *blocked
on you* and from the Interruption that says so.

## Acceptance criteria

- [ ] An instruction recorded against a Conversation comes back off its Timeline
      with its markdown intact, covered by a store round-trip test in the shape
      the neighbouring Event kinds have
- [ ] The viewer draws it as its own Timeline row summarising the instruction,
      off a generated wire type rather than a hand-written one
- [ ] `CONTEXT.md` gains **Manual Task** and **Stalled** entries, each with the
      _Avoid_ list that keeps it apart from the words above
