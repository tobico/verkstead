# 01. Several pull requests in one repository

## What to build

A Conversation can hold more than one pull request in one Repo, because a stack
is several in one. Everything Verkstead knows about a pull request is keyed one
per repository today — the record of what was opened, how its checks are getting
on, whether GitHub can merge it, and where it has got to — so each of those is
rekeyed off **the pull request** itself, the Repo staying on the row as the fact
it is.

Each is a rebuild rather than a widening: the rule is declared inline in the
table's own `CREATE TABLE`, so it is the table that has to be written out again
with the rows copied across and swapped in, run once as the database opens.
`pull_requests_by_repo` and `wrap_up_settled_by_repo` are the worked examples to
follow, and each is written to be safe against a database that has already had
it — what says whether there is anything to do is the presence of what it
rewrites, rather than a version number kept somewhere.

The keys, which are the decision rather than the code:

    pull_requests            UNIQUE      (conversation_id, repo_id, number)
    pull_request_checks      PRIMARY KEY (conversation_id, repo_id, number)
    pull_request_merges      PRIMARY KEY (conversation_id, repo_id, number)
    pull_request_standings   PRIMARY KEY (conversation_id, repo_id, number)

The rollup is the two-step the brief called it: it keys on the Conversation
alone, so it gains the Repo and the number together, and every row already there
is the Conversation's own repository's pull request — the only one it was
possible for it to be about. **And it is already wrong for a Conversation with a
read-write companion**: two watchers write one row and each reads the other's
suite. Putting that right is this slice's visible behaviour, and it is worth
being the thing the tests demonstrate.

**And the head branch goes on the row.** A pull request's head is a fact GitHub
told Verkstead when the work was taken up and nothing writes down, and two
things after this need it: holding a green rollup against what origin is holding
is a question about *that pull request's* branch rather than about whatever the
Worktree has checked out, and the stack session has to be told its branches. A
row written before this has no head to give, and the column stands empty there —
which for the Conversation's own repository is its branch.

Every read that assumed one pull request per repository moves with them: the read
of one, the list of them, the sweep after Done that asks where each has got to,
and the marks the Conversation view draws at each pull request's own Event. The
refusal that keeps one Conversation per piece of work stays as it is here — task
03 is where a stack gives it something new to say.

## Acceptance criteria

- [ ] A Conversation holding three pull requests in one Repo reads all three
      back, each with its own check rollup, its own merge reading and its own
      standing.
- [ ] A Conversation with a read-write companion draws each pull request's own
      rollup, where today the second written stands for both.
- [ ] Every recorded pull request carries the head branch its work is on, and
      the sweep after Done asks GitHub about each pull request rather than one
      per repository.
- [ ] A database written before this opens, keeps every row it held, and opens a
      second time with nothing left to do.
