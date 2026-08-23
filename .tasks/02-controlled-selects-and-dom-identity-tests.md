# 02. Controlled selects and DOM-identity tests

## What to build

Close the select-value divergence and pin the whole no-reset behaviour with
regression tests, so the bug class cannot quietly return.

The divergence: when a `<select>`'s options rebuild and the selected one goes
away, the browser snaps the visible selection to the first entry — but the
signal behind the select still holds the old choice, and Solid never re-applies
`value` because the string didn't change. The human sees one repo and creates
the conversation in another. Task 01's reconcile keeps option nodes alive in
the common case; this task makes divergence impossible in every case: the
displayed value is always re-applied after the option set changes, and a
selection whose option has genuinely vanished resets the signal explicitly, so
what is shown and what would be submitted can never differ. Apply the same
treatment to every server-data-backed select — the new-conversation repo
picker, the two profile pickers (details pane and manual-task composer), and
any others found.

Then the tests: a vitest helper that renders a component, drives a refetch of
its query the way a Nudge would, and asserts DOM node identity across it.
Pin the three named victims — conversation rows (the spinner element
survives), the repo picker's options, the profile pickers' options — plus the
select-value guarantee: selection shown equals value submitted, including
after the selected entry disappears from the payload.

## Acceptance criteria

- [ ] No server-data-backed select can display one value while its signal
      holds another — including when the selected entry vanishes on refetch
- [ ] A refetch-driving test helper exists and reads naturally in the
      existing vitest + golden-fixture setup
- [ ] Identity-across-refetch tests cover conversation rows, the repo picker
      and both profile pickers, and fail if reconcile is removed from any of
      their queries
- [ ] A test covers the vanished-selection case end to end
