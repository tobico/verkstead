# 01. A pull request per repository

## What to build

A Conversation can hold more than one pull request — one per repository — and
everything that reads one learns there may be several.

**One row per Conversation and Repo.** The pull requests table is unique on the
Conversation alone today, which is the rule *a Conversation is one branch and a
branch is one pull request* written into the schema. It becomes the
Conversation and the Repo: the repository is a column on the row, and the
unique index is the pair. The rows already there are the Conversation's own
repository's, which is what it was possible for them to be.

**That reshape is a rebuild rather than a declaration.** SQLite will not drop a
`UNIQUE` by `ALTER TABLE` — the constraint is part of the table's own text —
and the store's `CREATE TABLE IF NOT EXISTS` does nothing at all to a database
that already has the old shape. So it belongs with the one-time rewrites the
store runs as it opens: a new table, the old rows copied across and attributed,
the old one dropped, the new one renamed. That module has done exactly this
before, for exactly this reason, when a commit gained its repository — follow
that, including its rule about writing the shape out here rather than borrowing
it from the declaration. Safe to run twice, like the rest of it.

**Recording one is still the move into Wrapping**, and recording a second
against a *different* repository is not a second move — it is the same wrap-up
learning about another pull request. What must not change is the rule that
makes a second attempt at the same ending safe: a pull request recorded against
a repository that already has one reuses the row it has, which is also what
makes the second wrap of a split-out review land on the pull request it already
had rather than a duplicate of it.

**The readers split into two kinds**, and telling them apart is most of the
work. Some genuinely mean *the Conversation's own repository's pull request* —
the steer that will not move work into Wrapping without one, and the runner's
two readings of *is this branch already on a pull request*. Those keep asking
about one and gain the repository they meant all along. The rest mean *every
pull request this Conversation has*: the pinned block above the Timeline, and
the watchers of later tasks.

**The Event and the pinned block name their repository.** A pull request Event
carries which repository it was opened in, the pinned block above the Timeline
draws every one the Conversation has rather than the last one it finds, and a
companion's card says which repository it belongs to — where the Conversation's
own draws unlabelled, the same rule a commit's label follows: an unlabelled
card means the work's own repo, and the label earns its place when repos mix.

**The details pane asks GitHub in the pull request's own repository.** It takes
the repository off the Conversation today, which for a companion's pull request
would be asking the wrong one about a number that means something else there.
It follows the repo the pull request was recorded against.

## Acceptance criteria

- [ ] A database written before this opens with its pull request carried across
      and attributed to the Conversation's own repository, and opening it twice
      rewrites it once.
- [ ] Two pull requests stand against one Conversation, one per repository, and
      recording a second against a repository that already has one reuses that
      row and moves nothing twice.
- [ ] The pinned block draws every pull request the Conversation has, a
      companion's named with its repository and the Conversation's own
      unlabelled, and opening either asks GitHub in the repository it belongs
      to.
- [ ] A Conversation with one pull request looks and behaves exactly as it does
      now, the steer into Wrapping and the runner's re-entry included.
