# 02. The plain relay

## What to build

`/api/ui/members/{device}/…` on the workbench listener: this device taking an
ordinary call for one of its members, putting it to that member over the Peer
Listener **verbatim** — method, path, query, body and the headers that matter —
and handing the answer back untouched, status and body and all. The browser
stays same-origin and knows nothing about it; what it asked for is what it gets.

The dial is the one this tree already makes to a member: this device's own
certificate presented, the member's fingerprint pinned, and every address that
member advertised tried in the order it advertised them. A member that has
moved is reached at its later address, and a dial that answered nowhere leaves
the row unreachable the way any other does.

**The body is streamed rather than held.** An attachment is an ordinary `POST`
here — raw bytes, `application/octet-stream`, the name in the path — and the
route at the far end carries a limit of its own and answers `413` over it,
which the composer reads by name. So the hop puts no limit of its own in front
of that: it passes the bytes through and passes the member's own refusal back,
rather than buffering a file to decide something the far end decides.

**A prefix of its own rather than a segment under `/api/ui/devices/`**, which is
already the Devices section's namespace — see ADR-0020, where the decision and
its reasoning now stand.

Two device ids are not dialled:

- **One that is no member's** is refused by name, saying this device knows no
  such device rather than answering as though the path were wrong.
- **This device's own** is refused by name too. Local URLs keep their shape, so
  nothing should ever ask; a relay that quietly answered its own id would be a
  second way of spelling every local call.

A member that is down is refused by name as well, naming the device and that it
answered at none of its addresses. The point of that criterion is what it is
*not*: a browser left hanging on a dial, or a page drawing a proxy's own error
document.

Nothing in the browser changes here either — the client learns the device in the
next task. The demonstration is a call made against the hub and answered out of
the member's store.

## Acceptance criteria

- [ ] A read and a press under a member's Device Id answer exactly what that
      member answered: a Conversation's view, and a Set answered through the
      hop settling on the member.
- [ ] An attachment upload goes through with its body streamed rather than held
      in memory, and a body over the far end's limit comes back as that end's
      own `413`.
- [ ] A Device Id that is no member's, this device's own id, and a member that
      answers at none of its addresses are each refused by name rather than hung
      on or answered as a missing path.
