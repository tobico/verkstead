# 03. A new Conversation branches from origin's fresh default tip

## What to build

Starting a grilling fetches from the Repo's remote before the base commit
resolves, and an unpicked base means origin's default branch rather than the
local one — so a fresh Conversation comes off what origin holds now, not
wherever the human's local default branch last stood.

The rule, as settled with the human:

- **Fetch first, always** — whether the base was picked or left to the default
  rule. A fetch moves only remote-tracking refs, so it never touches the
  human's own checkout or local branches. Put the fetch beside the other git
  plumbing in `worktrees.rs`, as a helper the later call sites (task 04) reuse;
  it needs to say three things apart: nothing to fetch (no remote), fetched,
  and failed (with git's own words).
- **An unpicked base resolves `origin/<default branch>`** where the repository
  has an origin carrying one, and keeps today's local rule where it does not.
  A repo with no remote has nothing to fetch and nothing to be stale against —
  it is never refused for that.
- **A fetch that fails refuses the press by name.** The human chose refusal
  over proceeding on stale refs. A new refusal variant beside the existing
  `GrillingStarted` ones (render crate), carried through the API and named in
  the web viewer the way `NoBaseCommit` and the rest are — offline or
  auth-gone is something the human can go and fix, and the refusal is what
  tells them.

A picked base still resolves exactly as picked — the fetch just means a picked
remote-tracking branch resolves to where it stands now. Reopened Conversations
are untouched: their base is frozen and `start_grilling` makes no branch for
them, so the fetch-and-resolve applies only where a branch is being made.

## Acceptance criteria

- [ ] Grill start on a repo with a remote fetches first; an unpicked base
      branches from origin's default tip even when the local default branch is
      behind
- [ ] A repo with no remote behaves exactly as today — no fetch, local default
      branch, no refusal
- [ ] A failing fetch refuses grill start with a variant of its own, and the
      web viewer names it like the other refusals
