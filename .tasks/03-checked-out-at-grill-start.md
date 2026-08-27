# 03. Checked out at grill start

## What to build

Pressing the grill button gives every companion a Verkstead Worktree of its own,
or refuses the whole start naming the companion it could not deliver.

**Always a Verkstead worktree; the human's checkout never enters a sandbox.**
The sandbox principle already excludes even the checkout the Conversation's own
Worktree was made from — a shared checkout would carry uncommitted state anyway,
and git refuses a second checkout of a branch that is already checked out.

**Read-only checks out detached** at the selected branch's resolved commit. That
is a new worktree shape: the primitive today only ever cuts a new branch, and
this one adds a worktree holding no branch at all. **Read-write always cuts a new
branch** from the selected base, exactly as the main repo does — working directly
on an existing branch was considered and rejected, because commits would land on
the companion's own `main` and the checkout collision would come back.

**Fetch, then resolve, then check the branch, then add** — per companion, which
is the main repo's own order and for the main repo's own reason: a
remote-tracking ref is only as fresh as the last fetch, and an unpicked base
means origin's default branch rather than this checkout's copy of it. A
repository with no remote has nothing to fetch and is never refused for it.

**Every failure refuses the grill start naming the companion** — the fetch
failed, the base would not resolve, the branch is taken, git would not make the
worktree. Nothing new gates the button: companion configuration is always
complete, so refusal at the start is the whole story.

**A refused start leaves nothing behind — no branch, no directory, for any
companion.** The cheapest way to hold that is to ask every question before
answering any of them: fetch, resolve and branch-check the main repo and every
companion first, and only then make the main worktree and each companion's. What
is left to unwind is then a `worktree add` that failed partway, rather than a
half-configured Conversation.

**The directory is named for the repo and the branch it holds**, as the main one
is, under the Data Directory. A read-only companion holds no branch, so what
names its directory is the base it was checked out at.

**Companion worktree records get room of their own.** The existing table is one
row per Conversation and stays that way.

**Close removes the companion worktrees and keeps their branches**, exactly as
it does the main one — a branch is cheap and may hold work worth reading. A
companion worktree that will not be removed stops the close the same way the
main one does.

## Acceptance criteria

- [ ] Starting a grilling on a Conversation with a read-only and a read-write
      companion checks both out — one detached at the resolved commit, one on a
      new branch cut from it — and records where each went.
- [ ] A companion whose fetch fails, whose base will not resolve, or whose
      branch is already taken refuses the grill start naming that companion.
- [ ] A refused start leaves no branch and no directory behind for any
      companion, including ones that had already been made.
- [ ] Closing removes every companion worktree and keeps every companion branch.
