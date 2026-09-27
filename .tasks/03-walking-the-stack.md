# 03. Walking the stack

## What to build

At Start, a **Fix Merge Issues** pointed at a pull request walks GitHub's chain
both ways from it — a base that is another open pull request's head, a head that
is another's base — and records every pull request it finds, in order from the
bottom. The Timeline note the take-up already writes says what the stack is.

Read through `gh` rather than through `gh stack`, whose registry is per-worktree
and may not exist: the Repo's open pull requests asked for once with each one's
head and base, and the chain assembled from that answer. A chain that leaves the
Repo is not followed — a head in a fork is not a branch this Conversation could
ever push to, which is the same refusal the named pull request already gets.

The named pull request stays the Conversation's own: it is the one taken up, its
head is the Conversation's branch and its Worktree is where the work happens, and
it is the one that the refusal keeping *one Conversation per piece of work* is
about. **The neighbours are recorded without being claimed.** In this repository
they usually belong to another Conversation each, that being what a stacked stage
is, and the whole reason for recording them is that they are watched — so the
refusal reads what a Conversation was pointed at rather than every row recorded
beside it, and the note says which of the stack another Conversation holds. That
is a fact a human should be told rather than one to find out later.

A lone pull request records one, and its note says what it says today.

**A bare branch is walked when its pull request arrives.** Start over a branch
records no pull request and sends one `submitting` session for the one nobody
opened, so there is no chain to walk at the press: the walk runs where that pull
request is recorded instead — the same walk from the second door.

## Acceptance criteria

- [ ] Naming the middle of a three-deep stack records all three, ordered from
      the bottom, and the Timeline note says what the stack is.
- [ ] A lone pull request records one, and a chain whose next link is in a fork
      or another repository is not followed.
- [ ] A neighbour another Conversation holds is recorded, and the note names it
      as that Conversation's; pointing a second Conversation straight at it is
      still refused by name.
- [ ] A bare branch's stack is walked once its `submitting` step's pull request
      is recorded, and every recorded pull request is watched from there.
