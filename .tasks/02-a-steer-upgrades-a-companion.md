# 02. A steer upgrades a companion

## What to build

A companion that came in read-only can be opened up mid-life, and the steer is
where it happens: the human ticks it up to read-write in the modal, and the
detached checkout is replaced by a branch of its own.

**An upgrade control on each read-only row** of the companion section, with the
branch-name field beside it — the branch to cut, mirroring the Conversation's
own until a name is typed, exactly as at draft time. A read-write row offers
nothing: it is already as open as a companion gets.

**One direction only.** Nothing offers read-only and nothing offers removal, and
a submit that asks for either is refused rather than obeyed — what a session was
once given is never taken back mid-Conversation.

**The upgrade is fresh, not pinned.** The commit the read-only checkout was
detached at is where that repository stood when the Conversation started, and
the companion is joining the work *now*. So the upgrade fetches that
repository, re-resolves the branch its row names — or that repository's default
branch as origin holds it, where the row names none — cuts the new companion
branch from that tip, and replaces the detached worktree with one on the new
branch.

Replacing rather than adding beside: one companion is one checkout, and the
detached directory has nothing left to be. What is removed is the directory,
which is all a detached checkout is; nothing in that repository is otherwise
touched.

**Asked before anything is done**, as everything else in a steer is, and
refused naming the repository. A branch already taken is the refusal an upgrade
meets most — the name mirrors the Conversation's branch, which somebody may
have used in that repository already — and a fetch git would not make and a base
that resolves to nothing are the other two. Any of them refuses the whole steer
and leaves the companion read-only with its checkout exactly where it was.

**The row moves in the steer's transaction**: its mode to read-write, its branch
to whatever was typed or empty for mirroring, and its worktree and base commit
written over the ones the detached checkout had. From that moment it is a
read-write companion like any other — swept for commits, which reach the
Timeline labelled with its Repo's registered name; bound writable into every
session's sandbox; and expected to carry a pull request of its own at the finish
where the work touched it.

**And the Timeline says so**, beside what task 01 says about an add: the line
under the Steer names each companion upgraded and the branch it was given.

## Acceptance criteria

- [ ] Upgrading a read-only companion in the modal replaces its detached
      checkout with one on a branch cut from its base's tip as that stands at
      the moment of the steer, and the session the steer launches may write in
      it.
- [ ] The new branch is what was typed, or the Conversation's own branch name
      where nothing was, and commits landing on it reach the Timeline labelled
      with the Repo's registered name.
- [ ] An upgrade git will not make — a fetch that failed, a base that resolves
      to nothing, a branch already taken — refuses the whole steer naming the
      repository and leaves the companion read-only with its checkout where it
      was.
- [ ] No downgrade and no removal is offered anywhere, and a submit that asks
      for either is refused rather than obeyed.
