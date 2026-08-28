# 04. Steer and stages

## Goal

Companions can join and open up mid-life, and roadmaps carry them: the Steer
modal adds companions and upgrades read-only ones to read-write on any target
that launches a session, and a roadmap stage starts with its parent
Conversation's companion set already in place.

## Decisions in force

- **The Steer modal gets a companion section on every running target**
  (Grilling, Implementing, Wrapping) — it is sandbox setup for the sessions to
  come, not a property of one state. Settled Q14.
- **A steer may add and upgrade, never remove or downgrade.** Settled Q4: the
  frozen set only widens, the way the sandbox story stays simple — what a
  session was once given is never taken back mid-Conversation.
- **An upgrade is fresh, not pinned**: fetch, re-resolve the selected branch,
  cut the new companion branch from that tip, and replace the detached
  worktree. The companion is joining the work now, so it starts from now
  (settled Q15). The new branch name mirrors or is typed, as at draft time.
- **Adding at steer time follows the grill-start shape** — fetch, resolve,
  cut, bind — including the fetch the main-repo steer path deliberately skips:
  a companion added by a steer is new work joining, not an old worktree being
  put back.
- **Stages inherit the companion set** through the one inheritance funnel that
  already copies the Pairings: read-only companions as-is, read-write ones
  cutting a fresh companion branch per stage, named after the stage's branch.
  Without this a roadmap grilled with companions would build without them,
  stages having no draft moment of their own (settled Q9).

## Proposed tasks (provisional)

1. **Steer UI** — the companion section in the modal: current set shown,
   add rows for the rest, upgrade toggles on read-only ones; submit payload
   extended.
2. **Steer server path** — validate and apply adds and upgrades inside the
   steer transaction; create worktrees and replace detached ones beside the
   existing `somewhere` repair; refusals naming the companion.
3. **Stage inheritance** — copy the companion set where the Pairings are
   copied; per-stage read-write branch naming; stage refusals when a
   companion cannot be delivered.

## Re-verify at start

- Assumes stages 01–03 landed (inheritance is only worth having with the
  pipeline that wraps companion PRs).
- The steer form and payload in `web/src/workbench/Steer.tsx` and
  `crates/server/src/steering.rs` (`submit`, `somewhere`) — `somewhere` still
  skips fetching for the main repo.
- The stage inheritance funnel in `crates/server/src/continuing.rs::settle` —
  still copying exactly Pairings and brief.
- Whether stage 03 changed how companion branches are named or recorded.
