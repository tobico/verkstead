# 07. Record styling sweep

## What to build

The small settled visual calls, in one pass across the conversation list, the
timeline and the sheet:

- **Selection is the accent border alone.** The inset accent stripe retires
  everywhere it appears — the selected conversation card and the
  waiting question set alike (settled: the idiom goes whole, not per-site).
  A waiting set still carries its badge, so it keeps a mark; no background
  wash replaces the stripe.
- **Closed conversation cards dim to 0.45** opacity, down from 0.65, so Done
  and Aborted actually read as closed. Keep the existing correction that
  stops the meta line dimming twice, and keep the cards pressable.
- **Moves render centered, as the transition itself**: small, dim, and in
  arrow form — `Grilling → Implementing`. The record stores only the state
  moved to, so the left side comes from the move before it; the first move
  reads `Draft → Grilling`, an abort names the state it stopped in, e.g.
  `Implementing → Aborted`. The verb phrasing ("Started grilling") goes.
- **Task and stage lists draw checkboxes**: an empty box before a pending
  title, a checked box before a done one, in place of the state text on the
  right. The literal "done" / "to do" words stay in the markup but visually
  hidden, so screen readers and copied text still say the state — that the
  words travel in the DOM is deliberate and predates this change. Done items
  keep their dimmed title treatment.
- **The direction section styles as a question.** Where a question floats
  its orange label, the direction section floats the word **"End"** — the
  human's own choice of label — with the ask text running beside it, in both
  the settled record and the live chooser. No fake question number.

## Acceptance criteria

- [ ] No inset accent stripe remains anywhere; selection shows as the accent
      border, and waiting sets are still visibly marked by their badge
- [ ] Closed cards sit at 0.45 opacity, and every lifecycle move renders
      centered in `From → To` form, including the first and aborts
- [ ] Tasks and stages show checkbox glyphs with the state words present but
      visually hidden, and the direction section carries the orange floated
      label "End" styled exactly as a question's label
