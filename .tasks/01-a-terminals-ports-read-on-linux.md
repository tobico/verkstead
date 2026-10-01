# 01. A terminal's ports, read on Linux

## What to build

Today a Code pane terminal is a shell the member spawns inside the
Conversation's Sandbox, attached to by a socket the hub relays over the link,
and the only thing the server ever reads about its processes is the foreground
group's name, for the busy check. Nothing knows what a process in it is
listening on.

After this task **the member knows which devices hold an attach on each of its
terminals, and what ports those terminals have open.** Every request over the
Peer Listener already carries the calling member's certificate, so the terminal
register records, per terminal, the members whose attach sockets are open on it
— counted up on attach and down when the socket ends, as the file watcher's
register counts panes. A terminal attached only from this device's own browser
records nothing and is not read.

**While at least one member holds an attach**, the terminal is read about every
two seconds: the listening TCP sockets, on any address and either IP family,
held by any process in the terminal's pid namespace — the one bwrap makes under
`--unshare-all`, in which every process of the terminal lives whatever it did
with `setsid`. On Linux that is the socket inodes under each such process's
`/proc` fd listing, matched against the kernel's listener tables; it was proven
from outside a nested bwrap during the grilling, and the suite proves it the
same way. The read stops with the last attach. UDP is left out.

**The set of ports is reported two ways.** A `ports` Nudge, carrying the
Conversation and nothing else, whenever a terminal's set changes or the members
attached to it change — the hub turns a Nudge into a read. And a reading over
the link, per Conversation, listing the caller's own attached terminals with
their ports: a device that holds no attach reads an empty list, and a device
never sees ports of a terminal another device attached. The reading answers
only over the link; the device's own browser has no use for it yet.

A port that has gone leaves the next reading, within one tick. A terminal that
ends leaves it altogether.

## Acceptance criteria

- [ ] A server started in a terminal attached from another device is in that device's reading within two seconds, and gone within two seconds of ending; the `ports` Nudge is announced on each change.
- [ ] A terminal attached only from the member's own browser is not read; a device that holds no attach on a terminal reads none of its ports, and the reading is not served to the device's own browser.
- [ ] The pid-namespace walk is proven in the suite against a listener inside a nested bwrap, on Linux; the other platforms read nothing yet and say so in the code.
