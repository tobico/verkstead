# 02. The composer, and the session it starts

## What to build

The whole of a Manual Task working: the human types an instruction at the end
of the Timeline, picks a Profile, submits, and an agent does it.

**The composer** is the last thing in the Timeline — an auto-growing textarea,
a dropdown of Agent Profiles, and a submit — drawn in the slot the other
"what happens next" controls share, following the convention already there. The
dropdown defaults to the Conversation's implementation Profile, and a different
pick is one-off for that submission: it never updates the Conversation's own
implementation Profile.

It is offered whenever the Conversation has a Worktree, is not Draft or Aborted,
and **no session is registered for it**. That is the literal rule and it is
deliberate: the quiet gaps between auto-advance steps, a Wrapping lull,
Direction, Done, a run stopped on an open Interruption, and a Grilling
Conversation whose session has died all show it, because the point is to get a
stuck Conversation moving by hand. After a server restart no sessions exist, so
every Conversation shows it, and that is wanted too. Draft and Aborted have no
Worktree to run in, so they never show it.

Deciding that needs something the Conversation view does not carry today: a
working flag, filled from the sessions registry the way the sidebar's already
is, so a Nudge refreshes it.

**Submitting** posts the instruction. The server writes it to the Timeline as
the Event from task 01, takes the Conversation's Turn and holds it for the whole
manual session, and launches a session under the picked Profile inside the
ordinary sandbox on the Conversation's Worktree. Launching under a Profile the
caller names is new — the launch path fixes the implementation Profile today, so
the chosen one has to be threaded through.

**The prompt** is the new bundled skill named above the instruction and nothing
else: neither the Brief nor the handoff. The skill is Verkstead's own, embedded
and installed like the other eight, and its contents are pinned by assertions
the way theirs are. What it says: do what the instruction says; you MAY put a
Question Set to the human through the `verkstead` CLI if something genuinely
needs them, but nothing compels one if the task is already understood; commit
what you change. Implementation-flavoured, not grilling-flavoured.

**A submit that races a session loses.** If a session is registered when it
arrives, refuse it with a named outcome — an agent is already running — because
the composer that was pressed was stale, and an instruction written against a
stale world may no longer apply. Nothing is queued.

Holding the Turn is what keeps a driver from displacing the manual session:
starting a session displaces whatever is registered, so every launch has to
serialize on the Turn. The runner's launches do not today, and must.

**Ending it** is quiet plus no open Set: the session is ended once it has
printed nothing for a grace period *and* no Question Set of its own is awaiting
an answer. A manual task has no done-file to signal with, so quiet is the only
signal, and the grace must be distinctly longer than the runner's — a minute is
the working figure. A session idling on a blocking ask produces nothing for
hours, and the no-open-Set condition is what stops it being reaped.

**Nothing about the Conversation moves.** A manual run is Events on the Timeline
and the sidebar's working indicator while it runs: no state change, no re-entry
into Wrapping from a Done Conversation, and an open Interruption stays open with
*blocked on you* still on it. Commits land as Commit Events through the watcher
that is already running.

## Acceptance criteria

- [ ] The composer renders exactly when there is a Worktree, the state is not
      Draft or Aborted, and no session is registered; it goes while the manual
      session runs and comes back after
- [ ] Submitting lands the instruction on the Timeline and starts a session
      under the picked Profile, on a prompt naming the new skill above the
      instruction and carrying nothing else; the pick does not change the
      Conversation's implementation Profile
- [ ] A submit arriving while a session is registered is refused with a named
      outcome the page says in words, and a running manual session holds the
      Turn, so a driver wanting to launch waits rather than killing it
- [ ] The session ends on quiet plus no open Set — one idling on a blocking ask
      is never ended — its output and commits land as any session's do, and no
      Conversation state changes on account of it
