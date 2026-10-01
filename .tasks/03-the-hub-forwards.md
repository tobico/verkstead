# 03. The hub forwards

## What to build

The device whose browser has the terminal tab open — the hub bridging its
attach — is the one that forwards, and this task is its register of
**Forwards**. End to end: a terminal on B attached from A's browser starts a
server on 3000, and `curl localhost:3000` on A reaches it.

**Driven by the member's Nudge.** The hub already holds a nudge stream to each
member and re-announces every member's news locally under the device; on a
member's `ports` Nudge the hub re-reads that Conversation's ports over the link
(task 01) and reconciles its Forwards against the reading. For every port in it
the hub holds a loopback listener on the same number — IPv4 and IPv6 loopback
both — and each accepted connection becomes one upgrade over the link (task 02),
bytes crossed both ways and either close crossing. Nothing is forwarded for a
terminal this device's own browser attached locally: there is nothing to cross.

**A port the hub cannot take is skipped and said.** Where the number is already
bound on this device — its own dev server, or another member's port of the same
number already forwarded — the Forward is recorded as *skipped* with that
reason, and tried again on the next reconcile; last-one-wins and remapping were
both rejected.

**A Forward ends** two seconds after the attach it stands on ends, so a pane
swap or a reload keeps it; when the port leaves the reading; when the terminal
ends; and when the member reads as unreachable, there being nothing to connect
to. Ending closes the listener; connections already crossing are ended with it.
The grace is the file watcher's, for the file watcher's reason.

**The hub's own reading and Nudge.** A reading, for this device's browser only,
listing every Forward — port, the device it reaches and that device's OS, the
Conversation, and whether it is forwarding or skipped with its reason — and a
`forwards` Nudge announced on every change. Neither is served over the link:
they are this device's own, held back the way the remote-access, devices and
push namespaces are.

## Acceptance criteria

- [ ] With a terminal on B attached from A's browser, a server started in it on 3000 is reachable at `localhost:3000` on A within a few seconds, over IPv4 and IPv6 loopback; closing the tab ends it after the grace, and a pane swap keeps it.
- [ ] A port A already holds is listed as skipped, *port busy here*, and every other port is forwarded as before; the skip is retried when the port frees.
- [ ] The reading lists each Forward with port, device, OS, Conversation and standing; the `forwards` Nudge is announced on each change; neither the reading nor the Nudge is served over the link.
