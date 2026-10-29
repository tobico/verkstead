# 01. The menu, and a pending MCP server

## What to build

On every waiting Question Set, whatever role asked it, the Attach control on
each Answer field and on the set-level comment box becomes the plus-menu the
composer already draws: *Attach file…* first, then each declared MCP server not
yet on the Conversation, then *Add a server…* leading where the composer's
does. The plain paperclip is gone from the sheet. *Attach file…* on an Answer
field attaches a file to that Answer exactly as the paperclip did; the comment
box's *Attach file…* row is task 06's and is not drawn yet.

Picking a server records a **pending addition** on the Set, under that
question's label, server-side, so it survives a reload and shows on another
device. It is drawn as a chip beside the Answer's file pills, in the shape the
composer uses for a server, with the copy rules of `docs/copy.md`. The chip's ×
takes the pick off. A settled Set, whether answered, locked or closed, refuses
adding and removing alike, with the refusal a file on an Answer gets.

Nothing is applied to the Conversation in this task. The Conversation's own
server set is unchanged by a pick, a launch sees nothing new, and the Response
does not yet mention it. The record is the thing: it is what tasks 03 and 05
read at Submit, and what task 07 carries on a Transfer, so shape it to hold the
kinds those tasks add (a companion with its mode, base and branch; an opened-up
companion; a device) and the state they leave it in (pending, applied, struck),
not servers alone.

Read ADR-0028 and ADR-0021 before starting, and the CONTEXT.md entries for
MCP server, Attachment, and Question Set. The server strip and write-back of
ADR-0021 are untouched: a pending server is a declared one, by name.

## Acceptance criteria

- [ ] On a waiting Question Set of any role, every Answer field and the comment box draw the plus-menu, and no plain paperclip remains; *Attach file…* on an Answer field still attaches a file to that Answer.
- [ ] The server rows list declared servers not already on the Conversation; picking one records it on the Set under that question and draws a chip, and the chip is there after a reload and on a second device.
- [ ] × takes the pending server off; a settled Set refuses both the add and the remove, server-side and on screen.
- [ ] A pick changes nothing on the Conversation: its servers are as they were, and a launch after a pick is written with the same Built Root.
- [ ] Server tests cover the record and the refusals; web tests cover the menu on every role's sheet and the chip.
