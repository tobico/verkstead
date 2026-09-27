# 02. The settling rule over several pull requests

## What to build

The other half of the rekey, over the wrap-up's own bookkeeping: what a wrap-up
is waiting on, and what it has spent getting there, keyed off the pull request
rather than the repository. Two more rebuilds through the same migrations, in the
same shape as task 01's:

    wrap_up_settled      PRIMARY KEY (conversation_id, repo_id, number, waiting_on)
    check_fix_attempts   PRIMARY KEY (conversation_id, repo_id, number, check_name)

So the checks, what has been said, and whether the branch merges each name the
pull request they are about rather than the Repo it is in. The review stays what
it is — one review across the whole of the work — and carries no pull request the
way it already carries no repository.

One watcher per pull request rather than one per repository: the checks watcher
and the comments watcher are started for every recorded pull request, and each
asks GitHub about its own number in its own repository. And the rule that ends a
wrap-up reads every recorded one: three pull requests in one Repo are three
suites, three conversations and three merges to wait on, so Done comes only when
the whole of that has settled.

**A green suite is held against that pull request's own branch.** The check
watcher tells a stale rollup from a fresh one by what origin is holding, read
today off the Worktree's current branch. Three pull requests in one Worktree make
that two wrong answers, so what is asked about is the head recorded on the row in
task 01.

**The goes over a conflict are deliberately left where they are.** They are
counted against the Conversation and the Repo, and a stack lives in one
repository — so that count already means *per stack*, which is exactly what task
04 wants of it. Write the reason down beside the key rather than moving it, so
the next reader does not take it for the one table nobody got round to.

## Acceptance criteria

- [ ] Done comes only once all three pull requests in one Repo are green and
      GitHub says each of them merges; one red check anywhere holds it.
- [ ] The same check name red on two pull requests in one Repo spends a count
      each, and the goes spent on one are not the other's.
- [ ] A green rollup settles only where origin is holding the head GitHub named
      for *that* pull request, whichever branch the Worktree is on.
- [ ] A database written before this opens with everything it had settled still
      settled, and the goes over a conflict still counted per stack.
