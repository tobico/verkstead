# 04. Stages and Adopt come off a fresh default too

## What to build

The same freshness rule at the other places the default branch resolves —
settled with the human as one rule everywhere, using task 03's fetch helper.

- **An unstacked stage's branch** (`continuing.rs`): when a stage does not
  stack on its predecessor, its branch currently comes off the local default
  branch. Fetch first and come off `origin/<default branch>` where origin has
  one, local default where the repo has no remote — the same rule as grill
  start. A stage starts unattended, with nobody at a button, so a failing
  fetch **halts with a Notice** naming it rather than refusing a press. A
  stacked stage is untouched: its base is the predecessor's branch, which is
  local work.
- **Adopting a roadmap** (`stages.rs`): the adoption page and the press read
  the roadmap at the default tip when the human picks no base. Fetch before
  resolving there too, so what is adopted is judged against origin's tip. The
  press is attended, so a failing fetch refuses it the way task 03's grill
  start does; keep the no-remote case as today.

Nothing else resolves a default branch — grill start (task 03), these two, and
nowhere further; the reading was done during planning, but verify it against
the code as it now stands before assuming.

## Acceptance criteria

- [ ] An unstacked stage started while the local default branch is behind
      origin branches from origin's tip
- [ ] A stage start whose fetch fails halts with a Notice naming the fetch,
      and starts nothing
- [ ] Adopt resolves against a fresh origin tip, refuses an attended press on
      fetch failure, and repos without a remote behave as today everywhere
