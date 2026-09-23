# Steer as a Timeline item

The Steer form comes out of its modal. A steer is often a lot of text, and a
modal that blocks the workbench while it is written is the wrong place to
write it: the human wants to come and go, read the Timeline, look at another
Conversation, and finish the form when they are ready. So the press that
opened the modal now writes a **pending steer** beside the Conversation, the
Timeline draws it as its last item, the form is that item's details pane, and
what is typed into it is saved as it is typed. Submitting freezes the whole
form into the Steer Event as today, so the item stays as the record of what
was chosen. ADR-0010's *Amended: the form is a Timeline item* section
(2026-09-23) records what was settled: the stop at the press stays, Cancel
leaves the Conversation stopped, Resume and Close discard the pending steer, a
second press selects it, and a pending steer counts as waiting on the human.

Five sequential slices, store outward. Each is demonstrable in the workbench
by itself and carries its own tests: the store and server suites for the
pending steer and the record, and the web suite for the item, the form and the
frozen record.

## Tasks

- [x] 01: The pending steer, out of the modal — [details](01-out-of-the-modal.md)
- [x] 02: The form saves itself — [details](02-the-form-saves-itself.md)
- [x] 03: What the rest of the workbench says about it — [details](03-around-a-pending-steer.md)
- [ ] 04: The record is the whole form — [details](04-the-whole-form-on-the-record.md)
- [ ] 05: The vocabulary — [details](05-the-vocabulary.md)
