# 05. Refinement

## Goal

The daily-driver polish that was deliberately deferred: deferred
(non-blocking) asks, usage-limit pausing with push notification, manual
sidebar ordering with attention markers, milestone notifications, and
reopening finished conversations with a new brief round.

## Decisions in force

From [docs/design/verkstead.md](../../design/verkstead.md):

- **Deferred asks** are the second ask kind: they never block the session,
  sit in the timeline awaiting answers, and their answers fold into a later
  session's prompt. Work blocks *only* on questions whose answers affect
  upcoming work — this is what makes that rule complete. Requires a CLI
  extension (Verkstead is free to break the protocol — no wire-compat
  obligation to askance).
- **Usage-limit pause**: a claude account exhausting its window pauses the
  conversation and push-notifies; resume on say-so or window reset. No
  auto-switching between profiles.
- **Sidebar: manual ordering**, with a marker icon and border on
  conversations needing attention.
- **Push notifications: needs-you + milestones** (PR opened, stage complete,
  conversation done) — extending the needs-you set that already fires.
- **Reopening**: Done conversations accept a new brief round; the new brief
  is a new frozen event; the worktree is recreated if already cleaned up.
- **No cap on concurrent sessions** — confirm nothing in stages 01–04
  accidentally serialized execution.

## Deferred from stage 04

The retirement pass — stage 04's last task — was cut short on 2026-08-21, and
what it did not do is recorded here because this is the stage that would pick
it up.

- **No repository has been taken through Verkstead end to end.** No instance
  has ever run outside the test suite: nothing under the state directory, no
  unit, no database. Standing one up and driving real work through it —
  Brief, grilling, a Direction, a backlog to empty, a finish that opens its
  own PR, a wrap-up that settles — comes **before** the five items below.
  This is the point where the pipeline meets a machine it was not written
  against, and what that turns up is the refinement this stage is for.
- **The old toolchain still executes this roadmap.** Stage 05 will be started
  by roadrunner inside `tobico-scripts/bin/sandbox` unless a Verkstead
  instance takes over first, which makes the run above the first thing to do
  rather than a thing to schedule.
- **roadrunner and the wrappers carry no deprecation notice**, deliberately:
  one user, who knows. Nothing to do unless that stops being true.

The switch-over itself is written down — [docs/adoption.md](../../adoption.md)
— so the run has a page to follow and to correct.

## Proposed tasks (provisional)

1. **Deferred asks** — CLI flag + server support; timeline treatment
   (unanswered-deferred distinct from blocking); answer folding into the
   next session prompt.
2. **Usage-limit handling** — detect exhaustion from session output; pause
   state + push notification; manual resume and window-reset resume.
3. **Sidebar ordering + attention markers** — drag ordering persisted;
   marker icon + border driven by the blocked/interrupted badges.
4. **Milestone notifications** — PR opened, stage complete, conversation
   done, wired through the existing push pipeline.
5. **Reopening rounds** — reopen action on Done conversations; new brief
   event; worktree recreation; timeline shows round boundaries.

## Re-verify at start

- Assumes stages 01–04 landed. Verkstead is **not** the daily driver yet —
  see [Deferred from stage 04](#deferred-from-stage-04).
- Assumes usage-limit exhaustion is detectable from claude session output —
  verify against the claude version in use then.
- Assumes the push pipeline (VAPID, subscriptions) still matches askance's
  shape after four stages of divergence.
- Revisit whether anything else surfaced during adoption belongs in this
  stage ahead of the items above.
