# 08. The comments and review watchers

## What to build

The **comments** and **review** watchers move into the wrap-up component.
Comments are read through the `forge` host function and dispatch `responding`
and `addressing` sessions as `comments.rs` and `responding.rs` do today, a
comment marked addressed by its GitHub node id through task 05's host
functions. The review runs under the Review role as `review.rs` launches it,
and its settle puts the checks back to waiting as now.

**Fix Merge Issues** is the wrap-up with these two watchers off — an option
its start passes where `narrows_the_wrap_up` is asked today — so no review
session ever launches for it (ADR-0020, *Fix Merge Issues*). Where the Forge
lacks review, that watcher is settled at once.

With this, every watcher runs in the component and `wrapping::watching` starts
nothing in-process.

## Acceptance criteria

- [ ] The comments and review tests pass unchanged, every session launched by the component
- [ ] Fix Merge Issues runs with the comments and review watchers off and never launches a review session
- [ ] A comment landing mid-wrap-up unsettles the comments watcher as it does today
- [ ] A wrap-up over a Forge said to lack review settles that watcher at once
