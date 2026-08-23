# 01. Pick rides the closing Set

## What to build

The direction choice moves onto the proposal Set itself, so answering the
closing Set is the whole of accepting: no second trip to the Timeline.

The proposal block shrinks to its two real parts — the recommended direction
and the rationale — and `accepted_by` goes away, along with the validation that
served it. The viewer injects the three-way direction chooser onto any Set page
whose Set carries a proposal: all three directions offered every time, the
agent's recommendation marked but never preselected, the rationale rendered
beside the choices, and the chooser itself stating the semantics (picking a
direction accepts the proposal and lets the agent proceed; anything else —
another answer, free text, questions left open — sends it back). The Preface no
longer has to explain the mechanics, and the grilling skill stops telling it to.

The Response gains an optional direction pick, carried as a field of its own
rather than an ordinary Answer, and delivered to the waiting agent in the ask
result the CLI prints. `verkstead guide`'s proposal section is rewritten to the
new grammar.

Server-side, a picked Response accepts the proposal and routes immediately
through the existing direction machinery — the same moves the standalone
chooser press makes today (end the grilling session, take the handoff, start
the direction's fresh session). The old tails and the pre-proposal handoff
timing are untouched here; this task only removes the second step.

## Acceptance criteria

- [ ] A Set carrying a proposal validates on direction + non-empty rationale
      alone; `accepted_by` is gone from the schema, the validator, the guide,
      and the grilling skill's example.
- [ ] The Set page for a proposal Set renders the injected chooser: three
      directions, recommendation marked and not preselected, rationale beside
      it, semantics stated by the viewer.
- [ ] Submitting with a direction picked accepts the proposal and starts that
      direction's pipeline with no further press; the pick appears in the
      Response the agent receives.
- [ ] Submitting anything without a pick sends the proposal back exactly as a
      non-accepting answer does today.
