# 09. Agent profile cards

## What to build

Two things on the Agent profiles section. The plus button on its heading row
sits vertically centred on the heading rather than on its baseline; the row is
shared by every section with a plus, so the fix lands once and every heading
row gets it.

And each profile card reads as the harness logo followed by the harness and the
profile name joined by an em dash: "Claude Code — Personal". A profile nobody
named reads as the harness alone, "Claude Code", with no dash and no default
name after it. The logo is the existing harness mark, drawn before the words
the way the picker rows draw it. The model chips go from the card; the models
stay on the pane. The broken-account error line on the card stays.

Tests: the profiles web tests cover the new summary for a named and an unnamed
profile and the absence of model chips. The design doc's settings block
describes what a Profile's card shows.

## Acceptance criteria

- [ ] Every settings heading row with a plus button has it vertically centred on the heading.
- [ ] A named profile's card reads as its harness logo then "Claude Code — Personal", and an unnamed one as the logo then "Claude Code".
- [ ] No model appears on a profile card, and a broken profile's card still says so.
