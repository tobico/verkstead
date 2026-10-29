# 11. Cache the workspace's own crates

## What to build

Task 10 left the timeout at fifteen. On run 40798711420 the legs took 7m11 to
10m34, and five of the ten went over ten by 7 to 34 seconds. Each of those was
set by its build: 3m02 to 5m19 on one commit, depending on the runner, and
every leg over ten built for 4m52 or more. The tests are no longer the cost.

The human chose the free lever left (Set 45, Q1): `Swatinem/rust-cache`'s
`cache-workspace-crates: true` on the Windows job's cache step. That keeps the
workspace's own crates in the cache beside its dependencies, so a commit
rebuilds only what it touched and what depends on it. Check what the
action's version (v2.9.2) does with it on a cache shared by ten legs under
one key. Check also whether the cache stays inside GitHub's per-repository
limit with the workspace's artifacts in it.

Measure it the way every lever has been. It needs two runs: one to fill the
cache, and one on a commit that touches the server crate, which is what most
commits do. Write a ledger row for each in the Windows job's comment.

Then decide the timeout the way task 10 did. If every leg's job came in under
ten minutes, change `timeout-minutes` to 10 and name the run that proved it.
If a leg is still over ten, **the timeout stays at fifteen and the branch
finishes there** (Set 45, Q2): say by how much it missed, and do not ask
again. If the lever did not pay, it keeps its row saying so, and the next
commit takes it back out, as with rust-lld.

## Acceptance criteria

- [ ] The Windows cache step carries `cache-workspace-crates: true`, or a ledger row says it did not pay and a later commit removed it
- [ ] The ledger carries a row for each run that measured it
- [ ] `timeout-minutes` is 10 with the run that proved it named, or 15 with the shortfall written in the ledger
- [ ] The Windows job's comment, `docs/development.md` and the pull request's description say what the timeout is and why
