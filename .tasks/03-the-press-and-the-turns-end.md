# 03. The press, the turn's end, and the Conversation across

## What to build

Go wired: the press records a request the driver acts on, rather than an action
the route takes. It is `verkstead done`'s pattern and Stop's — nothing is moved
inside the request, because the session running for this Conversation is part
way through a turn and a move that cut across it would leave the work wherever
the agent had got to.

So the press writes down *this Conversation is going to that device*, and from
then until it lands the Timeline reads **Transferring to** that device's name.
The driver sees the running session out the way Stop after the current task
sees one out — whatever is running runs to its own end, nothing new is started
— and then the move runs. Pressed with nothing running, there is nothing to see
out and the move runs where it stands.

What the move does in this task is the Conversation itself. The far end is
asked to take it, and creates a row of its own carrying: the Repo the stage 08
matching settled, the branch name, the lifecycle it is in, its Pairings
resolved to that device's own Agent Profile ids — a mirror row being a local id
there like any other — the **Rank** string verbatim, and the **birth key**
task 01 put on it. The Rank travels unchanged because it carries the device
that issued it and is distinct cluster-wide by construction: the row keeps its
place in the merged order, and a later drag re-ranks it through whichever
device now owns it.

Only once the far end has confirmed does the source mark its own copy
transferred. That confirm is the commit point, and it is what makes a failure
safe: a move that falls over before it stops the Conversation on the source
live and stopped, with a Notice naming what failed, and whatever reached the
far end is swept rather than left as a half-record somebody has to find. The
source's Worktree is left exactly where it is either way.

No Timeline, no branch and no Worktree cross yet — the Conversation arrives as
a row in the right state and nothing else. The tasks after this fill it.

## Acceptance criteria

- [ ] A press over a running session leaves that session to finish its turn,
      the Timeline reading *Transferring to* the target throughout, and the move
      runs only once the session is out.
- [ ] A Conversation appears on the far end in the lifecycle it was in, under
      the Repo the matching settled, at the same Rank string and under the same
      birth key.
- [ ] The source's copy is marked transferred only after the far end confirms;
      the sidebar then holds one row, the far end's, and the source's URL
      redirects to it.
- [ ] A move that fails before that confirm leaves the source live and stopped
      with a Notice naming what failed, sweeps what reached the far end, and
      leaves the source's Worktree untouched.
