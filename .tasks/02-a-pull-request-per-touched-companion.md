# 02. A pull request per touched companion

## What to build

The finish session carries every companion it committed in to a pull request of
its own, and Verkstead finds each one and writes it down.

**The finish sequence extends to the companions.** The three skills that end a
piece of work on a pull request — the one that finishes a backlog, the one that
builds inline, and the one that writes a roadmap — each read the repository's
own `docs/agents/git-workflow.md` and follow its review process. That extends
to: for each companion this work committed in, follow *that* repository's own
review process, in that companion's worktree. It is the same split as ever —
the push and the pull request are the session's, and what is Verkstead's is
knowing that it happened.

**Touched means commits beyond the base**, and it is asked of git in the
companion's own repository at the moment of the finish. Verkstead already knows
each companion's repository, its branch and the commit its base resolved to,
because the commit sweep is built on exactly those three. A read-only companion
is not asked at all — its checkout is detached and bound read-only, so nothing
can have landed on it — and a read-write one with no commits beyond its base is
ignored by the whole of wrap-up: no pull request expected, nothing recorded,
nothing waited on.

Asked of git rather than of the commits the sweep has already put on the
Timeline. The sweep should agree, sweeping once more as a session ends, but it
is a poller's record: one that failed on a busy repository would leave a touched
companion looking untouched, and Verkstead would silently expect no pull
request from it.

**Each pull request is recorded as it is found.** The Conversation's own
repository is asked first, and the record of its pull request is the move into
Wrapping as it has always been; each touched companion is then asked in its own
repository and recorded beside it. So a wrap-up that cannot find all of them
still shows the human the ones that exist, pinned and clickable, while they
sort out the one that is missing.

**A touched companion without a pull request stops the run**, with a Notice
naming the repository — the shape today's missing pull request already has, and
a deliberate stop: the work ran and left no pull request, so what is wrong is
out here rather than in a driver that went away, and looking again would find
the same missing thing. What was already recorded stays recorded.

**Which means a stopped wrap-up must never reach Done.** Every stop a wrap-up
can take today leaves something unsettled behind it — red checks, a review
nobody finished, a batch nobody answered — so the rule that ends a wrap-up has
never had to ask whether the run was stopped. A companion whose pull request was
never found leaves nothing unsettled, because nothing was recorded to be
unsettled about: the recorded pull requests could all go green and the
Conversation would sail to Done past its own Notice. So the rule that ends a
wrap-up asks whether the run is stopped, the way every watcher already does
before it dispatches anything.

## Acceptance criteria

- [ ] Each of the three finish skills says to carry every companion holding
      commits to a pull request the way that repository's own review process
      says, in that companion's worktree, and its tests pin it.
- [ ] A finish that opened a pull request in a touched companion has it recorded
      against that repository and pinned on the Timeline, beside the
      Conversation's own.
- [ ] A read-write companion with no commits beyond its base is ignored
      entirely — nothing asked of GitHub, nothing recorded, nothing waited on —
      and a read-only one is never asked at all.
- [ ] A touched companion left without a pull request stops the run with a
      Notice naming the repository, the pull requests already found stay
      recorded, and the Conversation does not reach Done while it is stopped.
