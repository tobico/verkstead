# 02. The far scheduler stands down

## What to build

A device that holds a copy of a stage whose home is elsewhere starts nothing of
that stage's roadmap. Today the stage's roadmap row crosses with the slice, so
the far device reads a roadmap with one stage recorded and the rest as unticked
boxes, and could start the stage's siblings itself.

There are **two readings that start stages**, and both stand down:

- **The settle's carry-on** — `continuing::carry_on` into `continuing::reading`,
  run when a wrap-up settles. (`continuing::settle` is the Pairings/companions
  inheritance funnel, not the scheduler; the brief named it by mistake.) On a
  copy away from home it starts nothing and says nothing about the roadmap —
  task 01's report is what makes home act.
- **The places look** — `places` walks `driven_roadmaps` on a free place and
  on a slow clock. A roadmap this device drives is one with a stage born here;
  rows that crossed with a copy away from home do not make this device a driver.

Home is the stage's birth device, as task 01 settled. A stage that has come home
is born here and wears no mark, so it drives again.

## Acceptance criteria

- [ ] A settle on the far device, with a sibling stage ready, starts nothing there.
- [ ] The far device's places look never starts a stage of a roadmap it holds only copies of.
- [ ] A roadmap whose stages never leave home runs exactly as it does today.
