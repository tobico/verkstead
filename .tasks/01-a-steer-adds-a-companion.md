# 01. A steer adds a companion

## What to build

The Steer modal lets the human put another registered Repo into the sandbox the
sessions to come will run in, and the steer checks it out as part of the move.

**A companion section on every target work goes on in** — Grilling,
Implementing and Wrapping. Not on Done: nothing runs there, so there is no
sandbox to set up and nothing a companion could be for. The section is sandbox
setup rather than a property of one state, which is why it is the same section
under all three.

It shows the set already there — each repository's name, its mode, the branch
its checkout came off — as something to read rather than to edit, and offers a
row per registered Repo that is not on the Conversation yet. What each of those
rows asks is what the setup card asks while a Brief drafts: the mode, the
branch the checkout comes off, and for a read-write one the branch to cut,
mirroring the Conversation's own until a name is typed. This is the one other
moment those questions can be asked, the setup rows having gone when the card
froze.

**Nothing here takes anything away.** No row offers removal and no mode switch
offers read-only, and a submit that asked for either is refused rather than
obeyed: the frozen set only widens, which is what keeps the sandbox story
simple.

**The making follows the grill start's shape**: fetch, then resolve, then check
the branch, then make — including the fetch that the steer's repair of the
Conversation's own worktree deliberately skips. A companion added by a steer is
new work joining rather than an old worktree being put back, so it comes off
what the remote is holding now. A read-write one is cut a branch of its own; a
read-only one is checked out detached at the commit its base resolved to.

**Every question is asked before any of them is answered**, as at a grill
start, and all of it sits with the steer's other refusals — in front of the
session that gets ended, the stop that gets cleared and the worktree that gets
rebuilt. So a steer refused for a companion is a press that did not happen:
no directory, no branch, no row, nothing ended and nothing cleared. Each
refusal names the repository, because which one is the whole of what the human
needs.

**What is missing is made again, for companions too.** Two sources arrive with
companions configured and nothing on disk: a Draft, whose companions were
recorded on the setup card and never checked out, and a Conversation steered
back out of Closed, whose checkouts were removed and whose worktree rows were
forgotten while its branches were kept. Both would otherwise reach a running
state with companions the sandbox skips in silence — a session quietly missing
the repository it was given. So the steer makes every companion checkout the
record says is missing, beside the ones just added and beside the Conversation's
own: a read-write companion is checked out on the branch it already holds where
that branch is still there and cut one where it is not, and a read-only one is
detached at its base resolved at this moment, that being the only commit
anything can still name.

**The rows and the checkouts are written in the transaction that moves the
work**, the way the Worktree and the base commit the steer had to make already
are: a Conversation that said it had moved without saying where its companions
went would be one nothing could bind into a sandbox and nothing would come back
and remove. Past drafting is exactly where this writes, so it does not go
through the setup card's own guarded writes.

**And the Timeline says what was added.** The Steer is the human's own Event;
under it goes a line naming each companion the steer put in and the mode it
went in at. What a Conversation was configured with is read on the Brief's
details pane ever after, and this is what says when the set changed and who
changed it.

## Acceptance criteria

- [ ] Steering a Grilling, Implementing or Wrapping Conversation with a
      companion added checks it out, records the row and the worktree in the
      same act as the move, and the session the steer launches is bound to it
      at its mode and told it is there. Done offers no companion section.
- [ ] The add asks git in the grill start's order — fetch, resolve, branch — and
      a fetch that failed, a base that resolves to nothing or a branch already
      taken refuses the whole steer naming the repository, leaving no directory,
      no branch and no row, with nothing ended and no stop cleared.
- [ ] A companion the record holds with no checkout is made again beside the
      Conversation's own: a steered Draft's companions and those of a
      Conversation closed and steered back are both checked out, read-write ones
      on the branch they kept.
- [ ] The Conversation's own Repo, a Repo that is not registered and one already
      a companion are each refused by name, no removal or downgrade is offered
      or obeyed, and the Timeline carries a line under the Steer naming every
      companion added and its mode.
