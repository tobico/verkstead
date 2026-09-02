# 02. The Trimmed mark

## What to build

Make a trimmed Conversation say so, everywhere the loss would otherwise read
as breakage. The grilling settled that trim is marked rather than silent — and
also that there is *no* advance notice: nothing anywhere says a cleanup is
coming, only that one has happened.

Carry the trimmed state on the conversation view over the wire, exported to
the TypeScript types with the rest. Draw it on the Conversation's own page,
beside where the archived state already shows — a word, not a banner; the
glossary's word is **Trimmed**.

An agent-output card whose drill-down is gone must degrade politely: the card
itself still renders from its kept summary, and opening it says the detail was
trimmed rather than showing an error or an inexplicably empty pane. The same
courtesy anywhere else the removed rows were reachable from (a transcript
view, if one is reachable from the workbench).

An untrimmed Conversation renders exactly as it does today.

## Acceptance criteria

- [ ] A trimmed Conversation names itself Trimmed on its page; an untrimmed
      one is pixel-for-pixel unchanged.
- [ ] Opening a trimmed agent-output card explains the trim instead of
      erroring or rendering empty.
- [ ] The generated TypeScript types carry the new field and the web build
      passes.
