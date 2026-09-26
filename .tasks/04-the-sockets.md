# 04. The sockets

## What to build

The three attach sockets relayed, so the two panes that hold one open work on a
remote Conversation exactly as they do on a local one:

- **A Conversation terminal** — a human's shell inside the far Sandbox, which is
  the one socket that carries traffic both ways.
- **The Code pane's file watcher** — which carries nothing at all in either
  direction. The socket being *open* is the whole message: the first one on a
  Conversation starts the watcher behind the `files` Nudge on that device and the
  last one closed stops it.
- **A session's Screen** — what the agent's terminal shows as it prints.

The hop is a socket on each side of this device, and what matters is not how the
frames are carried but that the two live and die together. Verbatim at the frame
level and verbatim at the byte level are both open; a byte-level bridge over the
upgraded connection is worth weighing first, since it needs no WebSocket client
half in the binary and cannot paraphrase a frame.

**Both closes have to cross.** A browser that goes away — a tab shut, a laptop
whose lid closed mid-edit — has to take the far socket with it, or a watcher goes
on running on B over a pane nobody is looking at. And a member that goes away
has to close the browser's, or the pane sits attached to nothing with no way to
find out. The file watcher is what makes this load-bearing rather than tidy: it
is the one attachment with a cost on the other machine and no traffic to notice
its silence by.

A refusal at the far end is still a refusal: a terminal number that is not live
and a Conversation that device has no record of are answered as they are locally,
rather than turning into a socket that opens and says nothing.

## Acceptance criteria

- [ ] A remote Conversation's terminal echoes keystrokes, and a remote Screen
      paints as the session prints.
- [ ] A remote Code pane's attach starts the watcher on the member that holds the
      Conversation, and closing the pane stops it.
- [ ] A browser that goes away closes the relayed socket behind it, and a member
      that goes away closes the browser's.
