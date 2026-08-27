# 03. Checks per pull request

## What to build

Every pull request a Conversation holds has its checks watched, and a red one
gets a fix session that knows which repository it is fixing.

**A watcher per recorded pull request**, of the shape the one watcher has now:
it asks GitHub how that pull request's checks are getting on, in that pull
request's own repository, for as long as the Conversation is wrapping up and no
longer. What starts a wrap-up's watchers starts one of these per pull request —
the finish step, a server coming back up over a Conversation it left wrapping,
and a Resume alike, each of which starts the whole of a wrap-up rather than
some of it.

**Two attempts per check per pull request.** The count is kept per check today,
so that a suite where one job fails and is fixed and then a different one fails
has not spent its attempts. It gains the repository for the same reason: the
same check name red on two pull requests is two different failures, and one
spending the other's attempts would stop a run that still had somewhere to go.
That is another primary key growing a column, so another rebuild in the store's
one-time rewrites — the rows already there are the Conversation's own
repository's.

**Sessions still run one at a time.** Two red pull requests do not collide,
because a fix session takes the Conversation's Turn exactly as it does now and
a second queues behind it. What the watcher tries for and cannot get, it comes
back to later.

**The fix session is told which repository, which pull request and where to
work.** A session starts in the Conversation's own worktree and `gh` reads its
repository from where it runs, so a session sent at a companion's pull request
would otherwise ask the wrong repository how its checks are getting on. The
feedback names the repository, the pull request and the directory to work in —
the companion's worktree is bound into the sandbox already, so it is a
directory the session can simply work in.

**The addressing skill is written for that.** It says the branch is already
pushed and already has a pull request open, it pushes at the end, and it says
not to touch any other branch. Each of those has to hold when the branch,
the pull request and the worktree are a companion's rather than the
Conversation's own — including what *any other branch* means once a session may
legitimately be working outside the worktree it started in.

**Done waits for every pull request's checks.** The wrap-up settlements are
kept per Conversation today; the checks settle per pull request instead, and
the rule that ends a wrap-up expects a settled checks row for every pull
request on the record. The review's settle stays one, being one review. A third
rebuild in the store's rewrites, and rows already there are the Conversation's
own repository's again.

## Acceptance criteria

- [ ] Each recorded pull request has its checks asked about in its own
      repository, and a red one dispatches a fix session that names the
      repository, the pull request and the worktree to work in.
- [ ] The same check name red on two pull requests gets two fix sessions each
      before the run stops, and the stop's Notice says which pull request would
      not go green.
- [ ] Two red pull requests queue rather than collide: one fix session at a
      time, the second dispatched once the Worktree is free.
- [ ] A Conversation reaches Done only once every recorded pull request's checks
      are green, and a database written before this carries its attempts and
      settlements across.
