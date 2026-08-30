# 01. Give the share document the app's column

## What to build

The downloaded share draws the cards of a Set — the Preface, each Question, a
commit's Message — with their inner padding collapsed, and the Diff's
line-number gutter collapsed to nothing. The workbench draws the same record
correctly, from the same components.

Root cause, confirmed: the app wraps every page in `<main class="shell">` (the
`App` component), and the base stylesheet defines the column on `main` — the
`--measure` and, at wide windows, `--gutter` and `--bleed` custom properties,
plus the column's own width and padding. The share page mounts its panes with
no `main` at all, so every `calc()` built on those properties is invalid at
computed-value time and the paddings and gutter widths it feeds collapse to
zero.

Fix: the share page wraps what it draws in the same `main` shell the app wraps
every page in, so the two documents share one column definition rather than the
share growing a copy. Note the app's stylesheet already handles a shell that
holds panes (`max-width: none`, no padding) — the share must pick that rule up
too, which it does by using the same class, not by reproducing the values.

## Acceptance criteria

- [ ] A downloaded share of a Set with a Preface, Questions and a Diff draws
      card padding and the Diff's line-number gutter exactly as the workbench
      does — verified by eye against the same Conversation open in both.
- [ ] The share still opens correctly from `file://` and inside the viewer's
      sandboxed frame (nothing about the fix fetches or assumes an origin).
- [ ] A web test pins the share page to the same shell structure the app
      renders, so the wrapper cannot silently be dropped again.
