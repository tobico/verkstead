# 03. The boundary lines in the Capture

## What to build

On Windows, a session's boundary is a set of access-control entries written on
real directories before the agent runs. A first one took 112 s, and for all of
it the workbench showed a Conversation with a session starting and nothing
whatever to say why it was taking so long. The log alone is not an answer:
nobody opens a log from a phone.

So the session's Capture — its own record, and the one thing the human can
already open — carries two lines. One when the boundary write starts, saying
that it is being written and over how many entries. One when it ends, saying how
long it took. Written only on the platform that writes entries: a Linux or Mac
session's Capture carries neither, its sandbox being a wrapper with no entries
in it at all.

**The opening line has to be readable while the write is still going.** That is
the whole point — a line that arrives only once the 112 s is over says nothing
the human needed during it.

**Which means the Agent run Event has to be opened earlier than it is.** Today
the launch builds the sandbox, spawns the child, and only then opens the Capture
the session prints into — so while the boundary is being written there is no
Event to write into. The Event moves ahead of the sandbox build.

**A launch that then fails keeps its Event, with the reason written into it.**
Today a session that could not start leaves nothing on the Timeline at all: no
sandbox to run in, no terminal, no prompt, a boundary that would not be written.
Each is logged and none of it reaches the human, which is the same complaint
this stage exists for. With the Event opened first, the reason goes in the
Capture and the Conversation shows what happened.

Take care over the two things that follow from moving it. The Event is stamped
as it opens with the name Verkstead chose for the session and with the Pairing
it runs under, and both have to still be in hand at the new point. And the
session register learns of a Conversation only once its Capture is open, which
is what keeps a Set arriving in the launch window from being read as some other
backend's — the guard that holds across the launch has to go on holding across
the longer one.

**And the Capture will hold a line no terminal printed, which is a first.**
CONTEXT.md defines the Capture as the session's terminal bytes, escapes and all;
everything in one today came off the pseudo-terminal, the Rescue's typed line
included, which reaches it by echo. This is accepted rather than worked around,
and CONTEXT.md says so: a Capture may hold Verkstead's own line about the
session, and the boundary lines are it.

Rejected, and worth not revisiting: a Notice, which is a stop and this is not
one; and the log alone.

## Acceptance criteria

- [ ] A Windows session's Capture opens with a line naming the boundary being
      written and over how many entries, and carries a second line naming how
      long it took, both before the agent's first byte.
- [ ] The opening line is in the store while the write is still going, so the
      Conversation shows it during a slow boundary rather than after it.
- [ ] A Linux session's Capture carries neither line.
- [ ] A launch that fails leaves its Event on the Timeline with the reason in
      the Capture, and a session that starts normally is unchanged — the Event
      still carries the session's name and its Pairing, and a Set arriving in
      the launch window is still read as the right backend's.
- [ ] CONTEXT.md's Capture entry says it may hold Verkstead's own line about
      the session.
