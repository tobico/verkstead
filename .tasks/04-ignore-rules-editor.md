# 04. The row editor in the GitHub and git author pane

## What to build

The human-facing half: a new section in the GitHub and git author settings
pane for the ignore rules, alongside the token and git author sections. A row
editor — one row per rule with an author field and a body field, a remove
control on each row, and a control to add a row — saving the way that pane
already saves, and reading back what was saved.

The section says what the rules do in a line or two: comments matching a rule
are never addressed by an agent; a rule's given fields must all match; fields
are regular expressions matching anywhere, case-sensitive. The wording follows
the pane's existing voice.

A refused save — an invalid pattern, or a rule with both fields empty — shows
the server's error at the offending row, and nothing is silently dropped or
reordered. Rows the human empties out entirely can simply not be sent, which
is how a rule is deleted.

The middle-pane card for the section needs no new warning state; the ignore
list being empty is the ordinary condition.

## Acceptance criteria

- [ ] Rules can be added, edited and removed in the pane, survive a reload, and show up in `config.yaml`.
- [ ] An invalid pattern or a both-empty rule shows its error at the row and leaves the stored settings untouched.
- [ ] The pane's other sections, and the other settings panes, carry the rules along untouched when they save.
- [ ] The web tests cover the section the way the existing settings sections are covered, including the route word if a new pane word is added.
