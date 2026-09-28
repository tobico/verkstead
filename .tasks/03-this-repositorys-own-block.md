# 03. This repository's own block

## What to build

`docs/agents/git-workflow.md`'s `### Stacking roadmap stages` block says how a
Stage **joins** a chain, where today it only says how one is stacked at the
start. That block is the repository's own mechanism and the only thing the
joining session is told to follow, so it is written before the skill and the
server are written around it — and it is written from what the extension
actually does rather than from its documentation.

What the block has to cover that it does not:

- **A branch that exists and is not at the top of the chain.** Today it covers
  two cases: a first stacked Stage adopting its predecessor as the stack bottom,
  and extending a stack that already exists. A joining Stage is neither — it was
  cut from the highest settled Stage, which may be below the top, and by the
  time it joins it has been rebased onto the top.
- **Adopting a chain of any depth in one command.** The installed `gh stack`
  adopts a whole chain bottom-first from one `init`, while the block documents
  `init` taking a predecessor and one new branch. That gap was deliberately
  worked around elsewhere in this codebase, in the words a stack-sync session is
  sent with — find that and reconcile the two rather than leaving them
  disagreeing.
- **What an adoption adopts is what the checkout holds.** Adopting a branch this
  worktree has not got *creates it empty* rather than refusing, and what follows
  force-pushes whatever was adopted — so an invented branch is somebody else's
  real work overwritten. The block has to say: fetch, and have a local branch at
  origin's commit for every branch of the chain, before adopting anything, and
  read the adopted chain back before going on.
- **The registry is per worktree.** Each Stage is worked in a worktree of its
  own and `gh stack` keeps its registry inside it, so every joining session finds
  no stack and adopts the chain afresh. That is the normal case rather than a
  fault.

**`gh stack submit --auto` stays the way a stacked branch's pull request opens.**
It pushes every branch in the stack and repoints the bases of the existing pull
requests, which is what keeps the chain a chain on GitHub; the Stages already
finished this way and nothing below is rewritten by it, because each of those
branches is pushed at the commit origin already holds. Keep `--auto`: bare
`submit` opens an editor an agent cannot drive.

**Prove it rather than write it.** Make a scratch repository with a chain three
or four deep, cut a branch off the middle of it, rebase that branch onto the top,
and adopt and submit — and write down what the commands actually did. What
cannot be proven here does not go in the block.

## Acceptance criteria

- [ ] The block covers a branch that exists and is not at the top of the chain,
      and says what to do about the chain's depth, the local branches an
      adoption needs, and the per-worktree registry.
- [ ] What it says about adopting a chain is what `gh stack` as installed
      actually does, proven in a scratch repository rather than taken from its
      documentation.
- [ ] The `### Finish sequence` and `### Updating a stack after review` blocks
      still agree with it, and the words this codebase sends a stack-sync session
      no longer contradict it.
