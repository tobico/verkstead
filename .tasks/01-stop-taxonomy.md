# 01. Stop taxonomy

## What to build

The stop record learns who stopped a run, and the waiting-on-you marks stop
covering manual stops. Today a human press and Verkstead's own brake are both
stored as `deliberate`, so the sidebar disc and the *Blocked on you* badge
treat every stop as something waiting on the human — which is only useful when
the stop was outside their control.

The stored `stopped_by` words become four. This encoding is the settled
decision, so it is spelled exactly:

- `circumstance` — a restart or crash took the driver away. Unchanged.
- `deliberate` — legacy rows written before this change. Cannot be told apart,
  and the human chose to read them all as manual: no waiting marks.
- `human` — new stops from a Stop or Force stop press.
- `verkstead` — new stops Verkstead decided on, out-of-window stops included
  (the `resets` column still rides only on those).

Display rule: `verkstead` and `circumstance` stops keep the full treatment —
sidebar disc, accent *Blocked on you* badge. `human` and `deliberate` stops
show neither; instead the conversation header carries a quiet **Stopped**
label, drawn in the *Waiting on checks* style but pressable, jumping to the
stop's Notice on the record the way the accent badge does.

Restart and Resume behaviour is unchanged for all four: everything but
`circumstance` stays stopped until Resume, and a restart still picks a
`circumstance` stop up unasked. Push behaviour is unchanged too — human stops
already send nothing.

The sidebar's `waiting` computation (the SQL `OR` over open Sets and
`stopped_at`) counts a stop only when its word keeps the marks; the wire
carries whatever the badge-or-label choice needs so the browser never guesses.

## Acceptance criteria

- [ ] A manually stopped conversation shows no sidebar disc and no *Blocked on
      you* badge; its header shows a quiet pressable **Stopped** label that
      selects the stop Notice
- [ ] Verkstead-decided, out-of-window and crash stops look exactly as they do
      today
- [ ] Rows stopped before the upgrade (stored `deliberate`) read as manual
- [ ] Resume and restart behaviour is unchanged for every kind of stop, with
      tests covering the four stored words
