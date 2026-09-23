# 02. The form saves itself

## What to build

Every field of the pending steer's form is kept on the server as it is typed,
so the human can leave the item, the Conversation or the device and find the
form as they left it. The pattern is the draft Brief's: a keystroke starts the
pause, the pause coming round or leaving the field saves, one save in the air
at a time, and a keystroke that lands mid-save is saved after it. Every field
goes through the one keeper — the target, the brief, the digest tick, the
instruction, the follow-up brief, the pairing, the interrupt tick and the
companion rows added and opened up — and a save carries the whole form, so the
row is never half of two states.

A save endpoint on the Conversation takes the form and writes it onto the
pending steer. It is refused by name where there is no pending steer or the
Conversation is gone, and a refusal stops the field for good and is said under
the form, as the Brief's is: the commonest is a submit or a cancel from another
device landing mid-edit.

On the way in the form is prefilled from the pending steer, and a field follows
the record until the first keystroke and itself after it, so a re-read landing
mid-sentence cannot take the sentence with it. A second device sees what the
first saved on its next read, and the profile list under the pairing is not
rebuilt under a half-typed instruction while a session talks behind it — the
web suite already holds that case for the modal, and it holds for the pane.

Submit still sends what is in the form rather than asking the server to freeze
the row, so a keystroke inside the pause is never lost to the press.

## Acceptance criteria

- [ ] Typing an instruction, opening another item, and coming back finds it
      there; so does a reload, and so does a second device on its next read.
- [ ] Every field round-trips through the save: target, brief, digest,
      instruction, follow-up, pairing, interrupt, and the companion rows added
      and opened up with their modes, bases and branches.
- [ ] A save against a pending steer that has gone is refused by name, the
      field stops saving and the refusal is said; Submit sends the form's own
      state.
