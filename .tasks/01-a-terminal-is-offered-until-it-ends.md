# 01. A terminal is offered until it ends

## What to build

The member's side of a sticky Forward, which on its own fixes the bug as it
was reported: a Forward that ends when the Code pane is navigated away from.

**The member remembers every device that has attached one of its terminals,
until the terminal ends.** Today the terminal register counts, per terminal,
the members whose attach sockets are open, and that count is three things at
once: what starts and stops the ports reading, what the ports reading is
answered with, and the gate a connection to a port is let through. Add a
memory beside the count — the devices the terminal is **offered** to — that
grows on a device's first attach and never shrinks while the terminal lives.
The ports reading and the connect gate answer on the memory; the count stays
for what still needs it below.

**The ports reading runs from the first attach to the terminal's end.** It
stops today when the last member's socket closes, clearing the ports with it.
Now it stops only with the terminal. Its cadence is every two seconds while
any device holds an attach on the terminal and every ten seconds otherwise,
and a printed `localhost` address hurries it in either state exactly as it
does today — the output still flows to the server-held terminal whether or
not a tab is drawing it. A device attaching or letting go still announces a
`ports` Nudge for the Conversation, since the hub reconciles on it; the hub's
own loop needs no change in this task, because a port that stays in the
member's reading is a Forward that stands.

**The unit is the terminal, not the port** (grilling of 2026-10-19, Q3). A
server restarted on a new port after the tab closed is offered too. Offering
only the ports seen while a tab was open was rejected.

**What ends it is unchanged on the member**: the port leaving the reading, or
the terminal ending — its tab's ×, the shell exiting, the Conversation
closing, the server stopping. The hub's two-second grace stays as it is
(Q7); it now covers a dev server restarting on a file change rather than a
pane swap.

The ports suite under the server's tests is where this is proven, and its
forwarding test already walks the whole path: rewrite its ending, which today
asserts the Forward ends once the tab closes, and add what is new. Keep the
module docs of the terminal register, the ports module and the hub's
forwarding module true to what they now do.

## Acceptance criteria

- [ ] A server started in a terminal on B, attached from A's browser, is still
      forwarded at A's loopback after the tab closes — connections still reach
      it — with no attach held, for as long as the server listens.
- [ ] Killing the server in the terminal ends the Forward within the grace and
      a turn; closing the terminal ends it too, and nothing listens at A's
      loopback on that port afterwards.
- [ ] A device that never attached the terminal is offered nothing: its ports
      reading does not list the terminal, and a connection to its port is
      refused by name.
- [ ] With no tab attached the reading turns every ten seconds, and a printed
      address still hurries it; with a tab attached it turns every two.
