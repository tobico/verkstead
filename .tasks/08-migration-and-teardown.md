# 08. The migration and the teardown

## What to build

The end of Interruptions. A one-time migration rewrites every stored
Interruption event into a Notice — the what-stopped, the why, and the
evidence surviving as the markdown body, the settled remedy noted where one
was chosen — and drops the interruptions table. Then the teardown: the
Interruption event type and its wire shape, the settle endpoint and the
Retry and remedy machinery behind it, the card, the remedies sheet and the
evidence drawer in the viewer, and the tests that exercised them. The stall
sweep, halts, Notices and Resume are the whole story afterwards.

The glossary follows the code: the Interruption, Remedy and Stalled entries
give way to entries for the halt, the stop Notice and Resume, and any other
document still describing remedies is brought along.

Migration is the reason this lands last: until it runs, Timelines holding
old Interruptions — open ones included — still need the old card and settle
path to work.

## Acceptance criteria

- [ ] A database from before the migration opens with every old Interruption
      readable as a Notice on its Timeline, open ones no longer blocking
      anything beyond their halt.
- [ ] No interruption-named code, endpoint, component or test remains, and
      the suites pass.
- [ ] The glossary and docs describe halts, stop Notices and Resume, with no
      remedy sheet left in them.
