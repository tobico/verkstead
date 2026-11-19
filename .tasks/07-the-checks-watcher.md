# 07. The checks watcher

## What to build

The **checks** watcher moves from `checks.rs`'s tokio task into the wrap-up
component: polled every 30 s (`ASKED_EVERY`) through the `forge` host
function, a fix session launched on a failing check (under the role and skill
it runs under today, the Turn already held as the wrap-up holds it), and fix
attempts counted per check name through task 05's host functions over
`check_fix_attempts`, so the counts survive a restart. Exhausting the attempts
answers `given-up` as task 06 set out. A follow-up that pushed puts the checks
back to waiting, as now. Where the Forge lacks checks, the watcher is settled
at once.

Once it runs in the component, the in-process checks watcher is gone from the
wrap-up's start.

## Acceptance criteria

- [ ] The checks tests pass unchanged, every fix session launched by the component
- [ ] Fix-attempt counts per check name survive a server restart mid-wrap-up
- [ ] A wrap-up over a Forge said to lack checks settles the watcher at once and never polls
