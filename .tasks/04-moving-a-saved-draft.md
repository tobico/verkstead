# 04. Moving a saved draft

## What to build

A saved draft's composer carries the same device select, and picking another
device **replays the draft onto it and closes the one here**. The human chose a
move over a select that reads settled: a draft started on the wrong machine is
worth moving rather than worth making again.

A move is the compose page's replay run against a Conversation that already
exists. The Brief and the branch name are read off the draft and written onto a
new Conversation on the other device, through the endpoints a draft's own
composer uses, and the attached files go with them.

**The Repo is picked as part of the move.** A Repo id is one device's own, and
matching Repos across a cluster by their `origin` URL belongs to a later stage —
so the move asks which of the target's Repos the work is in: the same Repo select
this stage already points at another device, with its Open and Create rows behind
it, so a target with no such repository yet is not a dead end.

**What cannot travel says so.** The **base** goes back to the target repository's
default-branch rule, which is what a Repo switch does with it anyway. The
**companions** are ids in the old device's registry and are left behind. The
**Pairings** are Profile ids on the old machine, so the new draft arrives showing
the target's own remembered pairings — its prefill — which is exactly what a
draft created there would have arrived showing.

**The files travel.** A file on a draft is bytes under the old device's Data
Directory, and the move reads each back off it and puts it on the new
Conversation through the route a paperclip uses. Reading one is a route that does
not exist yet: an attachment is written and removed over HTTP today and never
read back, so this adds a read for one attachment — streamed, named by what it
was stored under, in the viewer's own namespace like everything else and so
relayed with nothing of its own. A file the target will not take is one more
refusal to carry, not a move undone.

**The old draft is closed, and its Timeline says where it went.** Closing is what
the actions menu's own row already does; what is new is the words left behind on
the record — the device the work moved to and the Conversation it became there.
On the Timeline rather than in a toast, because the human who comes back to a
closed draft weeks later is the one who needs to read it.

**Nothing is undone by a refusal**, which is the rule the compose page's replay
already holds: the new Conversation is real from its first request, a field the
target would not take leaves the rest of the work on it and is said on its
composer, and the old draft is closed only once the new one has been made. A move
that could not start the Conversation at all leaves both drafts exactly where
they were.

## Acceptance criteria

- [ ] A saved draft's composer draws the device select, and draws it only where
      another device exists.
- [ ] Picking another device asks which of that device's Repos the work is in,
      with Open and Create among the rows, and then creates the draft there
      carrying the Brief, the branch name and every attached file.
- [ ] The new draft's base reads the target repository's default-branch rule and
      its pairings read that device's own prefill; the companions are not carried.
- [ ] The old draft is closed, and its Timeline names the device the work moved to
      and the Conversation it became there.
- [ ] A refusal on the way leaves the new draft holding whatever the target took,
      says so on its composer, and leaves the old draft open.
