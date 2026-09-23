# 01. The pending steer, out of the modal

## What to build

Pressing **Steer** does what it does today — stops the drive, the ordinary
Stop, and reports whether a session is still running — and in the same press
makes a **pending steer** beside the Conversation where none is there yet.
One per Conversation, keyed by it, carrying when it was opened and a slot for
every field the form has: the target, the brief, the digest tick, the
instruction, the follow-up brief, the pairing, the interrupt tick, and the
companion rows added and opened up. This task makes the row and the slots;
task 02 is what fills them as the human types. A second press where one is
already pending makes nothing and reports the one there is.

The Conversation's view carries the pending steer, and the workbench draws it
as the **last item on the Timeline**, whatever lands under the press: it is
not a Timeline Event and it has no place in the record, so it is drawn after
everything that does. A card in the accent, reading *Steer* until a target is
picked and *Steering into X* after. It opens at an address of its own, a word
beside the share pane's and the terminal's rather than an Event id, and the
press navigates there — on a narrow window walking to the details pane, the
way selecting any card does. Landing on a Conversation with a pending steer
lands on it, as landing on one lands on the last openable item today. Reload
brings it back, because it is the server's. The address with no pending steer
behind it is the empty pane an unknown Event id gets.

The form that was the modal is now that item's details pane, drawn against the
live Conversation rather than one frozen at the press. Its fields stay page
state in this task. Its lead line still says the run has stopped while they
decide and that Cancel leaves it stopped. **Cancel** is a press now: a cancel
endpoint deletes the pending steer, and the page goes back to where the
Timeline was open before. **Submit** sends the form as today and lands the
Steer Event and the Moved line as today; the pending steer is deleted in the
same transaction as the record, and the page follows the record it wrote. A
refused submit is said under the form, as the modal said it.

The modal goes: nothing in the actions menu holds a form any more, and the
menu's Steer row is a press that navigates. The module docs on the form, the
menu and the server's steering module all describe the modal and the two
presses; rewrite them for the item as this task lands, since each later task
adds to them.

## Acceptance criteria

- [ ] Pressing Steer stops the drive, writes a pending steer, and opens it as
      the last Timeline item with the form in the details pane; pressing Steer
      again selects it without a second stop or a second row, and a reload
      draws it again.
- [ ] Cancel deletes the pending steer and leaves the Conversation stopped
      with Resume on offer; Submit lands the Steer and Moved lines as today and
      the pending steer is gone from the view.
- [ ] The store and server suites cover the pending steer's making, its second
      press, its cancel and its deletion at submit; the web suite's steer cases
      run against the pane, and none of them opens a modal.
