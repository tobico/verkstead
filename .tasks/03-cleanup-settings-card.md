# 03. The Cleanup settings card

## What to build

The **Cleanup** section of the settings, end to end, shaped like the
build-cache one: two rows — trim and delete — each a switch that saves the
moment it is flipped beside a days field that saves on a press of its own,
with the default drawn as a placeholder until a value is chosen.

**Config.** A cleanup section in the config file following the house pattern:
every field optional, an accessor per value saying the fallback — trim enabled
by default at 3 days, delete disabled by default at 30 — plus the
configured-or-default distinction the placeholder is drawn from. Nothing here
is ever an error: a missing, empty or unparsable value reads as nothing
configured. Settings are read afresh on every access, so the sweep picks up a
change without a restart.

**Wire.** The settings view and the settings edit both carry the section; the
whole-payload save echoes every other section unchanged, the way the existing
panes do.

**Web.** A Cleanup card and pane, and the section's word added where the
settings openings are declared so the route exists — forgetting that yields a
section that renders as no such page.

**Semantics settled in the grilling.** A delete duration shorter than the trim
duration saves fine — each clock counts from `archived_at` independently, so
the delete simply happens first; refuse nothing. Task 01's built-in three-day
constant moves behind the accessors here; the delete accessors are read by
task 04 and merely stored until then.

## Acceptance criteria

- [ ] Turning the trim switch off stops the next sweep trimming; turning it
      back on resumes; a typed duration changes what the sweep considers old,
      all without a server restart.
- [ ] The card round-trips through save with every other settings section
      unchanged, and the defaults show as placeholders until a value is
      chosen.
- [ ] The delete row saves and reads back, including a delete duration
      shorter than trim.
