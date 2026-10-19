# Sticky Forwards

A Forward — a port a terminal on another device of the cluster is listening
on, held on this device's own `localhost` at the same number — lived exactly
as long as this device held an attach socket on the terminal over the link.
The Code pane closes that socket whenever it is taken down, so navigating to
the Timeline, another Conversation or an Event ended every Forward two seconds
later. Nobody closing nothing meant that.

Now a Forward is held rather than attached (ADR-0019, *Forwards*, as amended
2026-10-19; CONTEXT.md, *Forward*). The member offers a terminal to every
device that has attached it, from the first attach until the terminal ends,
and what ends a Forward is the server stopping, the terminal ending, or a ×
on its row. Closing the tab, closing the page, the hub sleeping, the link
dropping and the hub's own restart all keep it: the hub reads everything a
member offers it whenever its stream to that member comes up or comes back,
and a member that stops answering leaves its Forwards standing as *device
unreachable* until it answers again.

## Tasks

- [x] 01: A terminal is offered until it ends — [details](01-a-terminal-is-offered-until-it-ends.md)
- [ ] 02: Everything a member offers, read back after a restart or a blip — [details](02-everything-a-member-offers.md)
- [ ] 03: A member not answering keeps its Forwards — [details](03-a-member-not-answering-keeps-its-forwards.md)
- [ ] 04: A × per row — [details](04-a-cross-per-row.md)
