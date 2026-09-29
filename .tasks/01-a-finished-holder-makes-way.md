# 01. A finished holder makes way for a take-up

## What to build

Start on a **Review** or a **Fix Merge Issues** is the take-up, and the take-up
refuses a pull request another Conversation has on its record, whatever state
that Conversation is in (`TakenUp::AlreadyHeld`). Narrow that refusal to a
holder that is still at work, and have a finished one make way. Read
[ADR-0020](../docs/adr/0020-a-conversation-has-a-process.md), *One open
Conversation per pull request*, first: it is the decision this builds.

This task is the **lone pull request**. A stack's neighbours are task 03, and
asking about uncommitted changes is task 02 — here a Done holder is closed
whatever its worktree holds.

What the holder's state decides, for both Processes alike and however the Target
named the work — a number, a link, or the branch the pull request is open on:

| Holder | The start |
|---|---|
| Done | closes the holder, then takes the pull request up |
| Closed, Archived | takes the pull request up; nothing to give up |
| anything else | refused leading there, as today |

**Closed by the ordinary Close**, the one the Close press runs: sessions and
terminals ended, the worktree and the companions' removed, the record moved,
open Question Sets shut. Not a second way of closing written beside it. It has
to be finished before git is asked about the branch, because a Done Conversation
keeps its worktree and the take-up refuses a branch that is checked out
anywhere — that second refusal is why lifting the first alone would not work.

**Every cheap refusal still comes first.** Nothing is closed for a start that
was going to be refused anyway for its Profiles, its Target, a fork or a pull
request GitHub has nothing open under. A take-up refused *after* the close — the
fetch failed, the branch diverged — leaves the holder Closed; a Steer brings a
Closed Conversation back, so say on the refusal's path in the log that it
happened rather than trying to undo a close.

**The lookup prefers the open Conversation.** A pull request is on several
records from here. Whoever asks which Conversation is on one — the take-up, and
the stack note that names a neighbour's holder — is told the one that is neither
Done nor Closed where there is one, and the newest otherwise. Today's query has
no order at all and would answer with the oldest.

**The refusal's words change**, since *one conversation per piece of work* is no
longer the rule: the sentence says the other Conversation is still at work on
that pull request, and keeps its link there. It is drawn in two places in the
web app, the plain sentence and the one with the link.

**The new Conversation's Timeline says what it closed**, naming the Conversation
by its branch, the way the stack note names a neighbour.

## Acceptance criteria

- [ ] A Review and a Fix Merge Issues over a pull request whose holder is Done
      both come back `TakenUp`, the holder reads Closed with no worktree, and
      the new Conversation is wrapping on the pull request's branch
- [ ] The same over the *branch* that pull request is open on
- [ ] A holder that is Closed or Archived is passed over and left as it was
- [ ] A holder in Grilling, Implementing, Wrapping, Follow-up or Investigating
      refuses the start leading there, and nothing is closed or made
- [ ] A start refused for its Profiles or its Target closes nothing
- [ ] Asked which Conversation is on a pull request two of them have recorded,
      the store answers the open one, and the newest where both are finished
- [ ] The refusal reads as a Conversation still at work, in both of its
      drawings, and the Timeline of the new Conversation names the one it closed
- [ ] The three existing `AlreadyHeld` tests are rewritten to the new rule
      rather than deleted
