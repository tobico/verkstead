# 07. CONTEXT.md and the ADR

## What to build

Write the new rule down. Read docs/copy.md before writing any of it.

Amend CONTEXT.md's **Carried Conversation** entry with the new occasion: a session
cut off mid-turn is carried on with the harness's own resume by whatever starts
the run again, and each in-scope way (sign-out, force stop, usage-window, stall,
crash, restart) gets its clause saying it is carried on, or stays Verkstead's own
Resume and why not — the way the entry already distinguishes Arrived / Widened /
Kept. Keep the **Resume**, **Stopped** and **Transfer** entries consistent with
it where they touch the same ground.

Write a new ADR under `docs/adr/` — "a cut-off session is carried on" — stating
the mid-turn rule and the carry-on-at-launch, cross-referencing the three it
generalises: ADR-0020 (Transfer), ADR-0028 (adding from an answer) and ADR-0029
(placing / *Keep it here*). Number it after the current highest.

The finish step opens the pull request after this task; nothing here opens one.

## Acceptance criteria

- [x] CONTEXT.md's Carried Conversation entry describes the new occasion and each in-scope way's outcome, in the register docs/copy.md sets.
- [x] A new ADR states the mid-turn rule and the carry-on-at-launch, cross-referencing ADR-0020, ADR-0028 and ADR-0029.
- [x] The Resume, Stopped and Transfer entries read consistently with the new rule.
