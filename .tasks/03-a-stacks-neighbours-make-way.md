# 03. A stack's neighbours make way

## What to build

The case this whole backlog was opened for: a Fix Merge Issues pointed into a
stack Verkstead built, where the conflict is in a pull request *below* the one
named. Every link of such a stack is a stage Conversation's, usually Done, and
each of those keeps a worktree with its branch checked out. The stack session is
told to sync the chain with `gh stack sync`, which rebases and force-pushes every
branch of it — and git will not move a branch that is checked out in another
worktree. Nothing in the dispatch or the `addressing` skill looks for that today.

So the neighbours make way by the rule tasks 01 and 02 built for the named pull
request. See [ADR-0020](../docs/adr/0020-a-conversation-has-a-process.md), *One
open Conversation per pull request*.

**The chain is walked before anything is recorded or closed.** Today the walk
runs where the pull request is recorded, after the take-up has made its
checkout. The press needs the chain first, to know who is standing on it. The
walk that records the stack stays where it is; what moves ahead is knowing the
links. A Process that does not walk a stack asks nothing here.

**Who is standing on a link** is the Conversation the pull request is on — the
lookup task 01 ordered — and also any Conversation whose worktree has that
branch checked out, which is what git will actually refuse over. A checkout that
is no Conversation's is the existing refusal naming where the branch is checked
out, said about the neighbour.

For each link's Conversation:

| It is | The start |
|---|---|
| Done | closed, by the ordinary Close |
| Closed, Archived | nothing to give up |
| still at work | refused, naming it, **before anything is closed** |

**All or nothing.** One neighbour still at work refuses the whole start with
every Done Conversation left Done. The refusal names the Conversation and leads
there, as the refusal over the named pull request does.

**One question for the whole stack.** Neighbours holding uncommitted changes
join task 02's list beside the named pull request's holder, so a stack of five
is asked about once.

**The Timeline says which were closed**, in the note that already says what the
stack is and whose each link was.

**A bare branch** takes up with no pull request recorded and walks once
`submitting` has opened one. Nothing can be asked of the human by then. Where
that later walk finds a neighbour's Conversation Done, close it if its worktree
is clean; where it holds uncommitted changes or is still at work, stop the run
with a Notice naming it rather than dispatching a sync that cannot move the
branch.

## Acceptance criteria

- [ ] A Fix Merge Issues over the middle of a three-link stack whose other two
      links are Done Conversations' closes both, and no branch of the chain is
      checked out anywhere but the new Conversation's worktree
- [ ] With one neighbour still at work the start is refused naming it, and the
      other neighbour and the named pull request's holder are as they were
- [ ] Neighbours with uncommitted changes are named in the same list as the
      holder's, and one confirming press closes all of them
- [ ] A neighbour that belongs to no Conversation is left alone and noted as
      today
- [ ] The Timeline note names the Conversations that were closed
- [ ] A lone pull request starts exactly as task 01 left it
- [ ] A bare-branch take-up whose pull request turns out to be a link of a stack
      stops with a Notice where a neighbour could not make way
