# Adding from an answer

A Question Set is where the human finds out the work needs more than the
Conversation was given: another repository, a declared MCP server, leave to
move to another machine. Today the only way to give the agent any of those is
to leave the sheet for the Steer form and come back. This replaces the
paperclip on every Answer field, and on the set-level comment box, with the
composer's plus-menu, so the addition is made where the question is answered.
A pick is recorded on the Set the moment it is made and applied in the
transaction that stores the Response; the Response carries an `added` list
beside `attachments` so the agent that asked learns what came with the answer;
and where the addition changes the sandbox and a session is waiting on that
very Set, the session is ended and carried on in a wider sandbox the way a
Carried Conversation is, the harness resumed with a note that names the Set.

The decisions, and the alternatives turned down, are in
`docs/adr/0028-adding-from-an-answer.md`. Each task below is a slice from the
store to the screen, and the order is the dependency: the record exists before
anything is applied, applying exists before anything is restarted. For two
tasks an addition reaches the next launch rather than the waiting agent, which
is the Deferred Ask's own rule and was accepted as the intermediate state.

## Tasks

- [x] 01: The menu, and a pending MCP server — [details](01-menu-and-pending-server.md)
- [x] 02: Pending companions and device ticks — [details](02-pending-companions-and-devices.md)
- [x] 03: Submit applies servers and ticks, and writes the record — [details](03-submit-applies-servers-and-ticks.md)
- [ ] 04: Submit applies companions — [details](04-submit-applies-companions.md)
- [ ] 05: The restart — [details](05-the-restart.md)
- [ ] 06: A set-level file — [details](06-set-level-file.md)
- [ ] 07: Crossing on transfer, and the words — [details](07-crossing-and-the-words.md)
