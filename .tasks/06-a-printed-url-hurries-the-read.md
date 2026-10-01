# 06. A printed URL hurries the read

## What to build

A dev server prints its address the moment it is up, and the two-second tick
is a long time to look at *cannot connect*. After this task a URL printed in a
terminal's output triggers an immediate read of that terminal's ports instead
of waiting for the tick — and nothing more: the print is a hint, and a port
nothing in the terminal's tree is listening on is not forwarded on the strength
of it (Set 999, Q13).

What counts is a URL naming this machine — `localhost`, `127.0.0.1`,
`0.0.0.0` or `[::1]` — with an explicit port, anywhere in the bytes the
terminal's screen receives, read where the output already passes on its way to
the Screen and the attached sockets. One hurried read per burst: a server that
prints its URL three times is read once, and the tick carries on as before. The
sniff runs only while the terminal is being read at all, which is while a
member holds an attach on it.

## Acceptance criteria

- [ ] A server that prints its `localhost` URL on starting is in the reading before the next tick would have run.
- [ ] A printed port that nothing in the terminal's tree listens on is not in the reading and nothing is forwarded for it.
- [ ] Output in a terminal nobody holds an attach on over the link triggers no read.
