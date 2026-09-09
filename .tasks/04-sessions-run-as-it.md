# 04. Windows sessions run as the account

## What to build

The switch. Everything the three tasks before it built is now what a Windows
session actually uses, and the AppContainer goes.

What a Conversation gets is no longer a container of its own. The module that
held one keeps everything about it that was never really about containers — the
per-Conversation entries, the record under the Data Directory written before
anything is granted, the sweep that takes a crashed-over Conversation's entries
back at the next startup — and loses the profile, the capability, and the two
questions that only made sense of a container (what container a process is in,
and what a process holds). It is no longer named for a container either: it
holds a Conversation's **entries**, and the account is its own module beside it.

Because there is one account for the installation, the identity is resolved once
rather than made per Conversation, and a Conversation's boundary is the set of
entries alone. Everything that a session may reach is unchanged; what a session
may reach *that is not its own* is now every live Conversation's Worktree, which
is the cost this stage accepted and which the entries coming off with a closed
Worktree bounds.

A session's process is started as the account, on a console the launcher makes.
The description a rendering carries names the account rather than a container.
The named pipe stops being a set that each container joins as it is made and
becomes one identity granted when the pipe is opened, because there is now only
ever one — which is a simplification rather than a retargeting.

**A boundary that cannot be made refuses the session**, exactly as before and
for the same reason: no account, no password, an entry that will not write.
There is no unsandboxed session to fall back to.

The boundary suite is what proves this, and it should ask the same questions of
the account that it asked of the container: a file written where the Surface
says read-write, refused where it says read-only or nothing, the pipe reached,
and — new, and the point of the whole stage — the human's own Documents refused
for reads as well as writes, and `bash` running.

## Acceptance criteria

- [ ] A Windows session starts, paints its Screen, and runs as the session
  account rather than in a container
- [ ] The boundary suite's write / read / refused classification matches the
  Surface for every kind of access it describes
- [ ] The human's Documents are refused to reads **and** writes from inside, and
  `bash` runs inside
- [ ] `verkstead ask` from inside a session lands a Set through the pipe
- [ ] No `CreateAppContainerProfile` remains anywhere in the tree
- [ ] A Conversation's entries come off with its Worktree, and a Done
  Conversation's entries are gone after a simulated crash and the next startup
- [ ] A machine with no account, or no password for it, refuses the session with
  a line saying which — and never starts an unsandboxed one
