# 04. Cleanup nests its days under two checkboxes

## What to build

The Cleanup pane's two switches, trim archived conversations and delete them
for good, become the native checkbox from task 02, with their labels unchanged.
Days after archiving before trimming nests under the trim checkbox and days
after archiving before deleting nests under the delete one, each with its Save,
each indented and greyed while its checkbox is off, the same pattern the size
field took on Language support.

The paragraphs go: the one at the top of the pane and the one under each
switch saying what trimming and deleting take. The line saying the delete
comes first at these durations is computed and stays. The card's summary is
unchanged.

Tests: the cleanup web tests move from switches to checkboxes and cover each
days field greyed while its checkbox is off.

## Acceptance criteria

- [ ] Each days field and its Save grey and refuse input while its checkbox is off, and take input once it is on.
- [ ] Ticking either checkbox saves at once.
- [ ] The card summary and the ordering warning are unchanged, and no static paragraph remains on the pane.
