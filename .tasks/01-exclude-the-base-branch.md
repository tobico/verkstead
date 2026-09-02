# 01. The sweep leaves out what the base branch already holds

## What to build

A commit belongs to a Conversation when the base branch does not already hold
it. Today the sweep lists `<base commit>..<branch>` and records everything in
it, so a resolution session that merges the base in puts every commit the base
gained since the work was cut onto the Timeline. After this, it puts none of
them there.

Two halves.

**The base branch's name gets recorded.** A Conversation keeps only the commit
its base resolved to, and a sha cannot say what branch it came off. A Companion
already keeps both — `base_ref` for the name the human picked and `base_commit`
for what it resolved to, with a doc comment giving exactly this reason — so a
Conversation grows the same pair, filled at the moment the work starts and at
every other place a base is resolved: starting a grilling, steering into one,
continuing, and a Stage's own start. Drafting is left exactly as it is; its
column goes on holding the picked name until start overwrites it with the sha.
The column is nullable and nothing backfills it: every Conversation alive today
has no name recorded and never will.

**The sweep excludes what that branch carries.** The listing becomes the
Conversation's commits minus everything reachable from the base branch:

    git rev-list --reverse <branch> --not <base commit> <base ref> <remote counterpart>

The recorded name and its `origin/` counterpart both, because an agent told to
fetch and merge may end up on either. No fetch is added to the sweep — it runs
every couple of seconds, and the resolution session fetches before it merges,
so origin's copy is already current by the time there is anything to exclude.

The fallbacks, in order: nothing recorded, or a recorded name that no longer
resolves, excludes by the Repo's default branch as origin holds it; a Repo where
that does not resolve either sweeps by `<base commit>..<branch>` exactly as it
does today. A ref that does not resolve is dropped from the arguments rather
than passed to git, which would fail the whole read.

A read-write Companion is swept the same way, off the `base_ref` it already has.

## Acceptance criteria

- [ ] A branch that merged its base branch in gains only its own commits and the
      merge commit on the Timeline — the base's own commits are never recorded.
- [ ] A Conversation started from a picked base branch has that branch's name
      recorded beside the resolved sha; one started from no pick records the
      Repo's default branch as origin holds it.
- [ ] A Conversation with no recorded name — every one that exists today —
      excludes by the Repo's default branch, and one whose recorded name has
      stopped resolving falls back to the same.
- [ ] A Repo with no resolvable base branch at all sweeps exactly as before.
- [ ] The migration adding the column is safe to run twice.
- [ ] `CONTEXT.md` says the rule where it defines what a commit Event is.
