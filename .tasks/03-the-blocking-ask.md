# 03. The blocking ask under OpenCode

## What to build

A grilling under an OpenCode Profile that puts a Question Set to the human,
holds its shell command open while they answer, and carries on with the Response
on stdout.

Most of that already works: the CLI asks the same way everywhere, and OpenCode's
channel is the blocking one it was given in task 01. What is missing is the
Guide — and one sentence of it is actively wrong for this backend.

**The Guide is tailored per channel, and has to become per backend.** The
blocking half of *Running the ask* names Claude Code and tells the reader to make
the call a background one, because until now Claude was the only backend that
blocked. OpenCode is the second, and it does not work that way: its shell tool
runs the command synchronously in the model's own turn, holding nothing open on
the model's side, so what an OpenCode session needs told is to **pass a large
timeout**, not to background anything. A session handed Claude's instruction
would look for a harness feature it has not got.

So the splice moves from the channel to the agent type. Claude's blocking
section stays what it is; OpenCode gets one of its own; the store-and-nudge
backends keep sharing theirs, because what they share is real. Everything about
*writing* a Set is common and stays written once. The environment already
carries the agent type into every sandbox for exactly this, and the Guide read
outside a sandbox stays the blocking one a human at a terminal has.

**Raise the default timeout as well as saying it.** The shell tool opencode
registers takes any positive timeout — which is what makes the blocking ask free
here — but its default is two minutes, and its own description tells the model
that commands time out after the default. A model that passes nothing would have
its ask killed two minutes in, hours before the human answers. opencode reads a
variable that raises that default; set it in the sandbox environment so a session
that ignores the Guide still holds. Both, rather than either: the instruction is
the mechanism and the variable is what keeps a drifted instruction from being a
wedged ask.

**A held ask is a quiet session, and nothing must end it.** With an at-work
signature, a session sitting on `verkstead ask` draws no at-work line and reads
idle — correctly, since it is doing nothing. What keeps it alive is that an
unanswered Set of its own counts as open, which every ender and the Rescue both
read. That is already built and is not this task's to write; it *is* this task's
to prove under this backend, because a blocking backend whose held asks got
reaped would lose the answer the agent asked for.

## Acceptance criteria

- [ ] `verkstead guide` printed inside an OpenCode sandbox gives OpenCode's own
      running-the-ask section; inside a Claude one it still gives Claude's; the
      two store-and-nudge backends still give theirs; and a Guide printed
      outside a sandbox is unchanged.
- [ ] A grilling under an OpenCode Profile blocks on `verkstead ask`, survives a
      gap far longer than the tool's own default timeout, and resumes with the
      Response as YAML on stdout — with the session's own turn never having
      ended and nothing typed into its terminal.
- [ ] A session held on an open Set is not ended by any ender or prodded by the
      Rescue while it waits, and is ended by the ordinary rules once the Set is
      answered and its work is done.
