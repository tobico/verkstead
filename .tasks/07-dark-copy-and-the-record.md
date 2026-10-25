# 07. Dark, copy and the record

## What to build

The pass over the whole page once every screen has landed, and the record of
what changed.

**Dark.** The dark boards are the light ones with the dark tokens applied
mechanically, so check them against the screen: the Installed, Missing, Failed,
Not installed and Optional pills on the code wash, the ember callout and the
green and red washes, the selected wash as a color-mix, and the rail on the
dark hunk. Nothing new is added to the palette; where a pill cannot clear
contrast on the dark code wash, it takes the wash one step up rather than a
colour of its own. Take screenshots at 1088px and 390px in both schemes,
driving the real viewer the way this repository checks layout, and compare
them by eye against the PNGs under `docs/design/setup-wizard/`.

**Copy.** Sweep every string the six tasks added against `docs/copy.md`:
Title Case on the h1 to h3 and sentence case on everything else, the
eyebrow, the group labels, the pills, the button labels, the reasons beside
disabled buttons, the reading line, the sheet, the callouts. British spelling,
*Cancelled*, the one-character ellipsis, no em-dashes on screen, full stops on
sentences and none on labels, the serial comma, and the human as *you*.

**The record.** ADR-0016 says things the wizard no longer does. Amend it, dated,
where the design changed a rule: every button in the wizard read Next and now
names what it does (Install N, Keep N profiles, Finish setup, Skip); the hint
screen was reached only after a run and is now reached from the choosing
screen over every missing row, with Next released on the step's objective
rather than on every listed row; the token stopped the wizard for one more
press whenever it was verified and now stops it only for a refusal or a
missing scope; and the onboarding reading carries the machine's name. Check
`CONTEXT.md`'s *Onboarding Mode* entry still describes the page, and that the
design folder's README still says truly where the grilling overrode the notes.

## Acceptance criteria

- [ ] The two dark boards match the screen at 1088px, and every pill and
      callout clears contrast on the dark tokens; the screenshots were taken
      and compared.
- [ ] Every string the wizard shows passes copy.md's rules, and the `Set Up
      Verkstead` heading is gone from the strings and the tests.
- [ ] ADR-0016 carries a dated amendment saying what the wizard now does, and
      `CONTEXT.md` and the design README read true.
