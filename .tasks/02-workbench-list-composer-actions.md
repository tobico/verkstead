# 02. The workbench's list, composer and actions

## What to build

Read `docs/copy.md` first; it is the whole specification of what a string
should say. Then bring the copy of the workbench's outer surfaces into line
with it: the conversations list and its rows, the compose page and the
composer, the setup row (repo, process, agent) and the roadmap adoption and
take-up cards, the actions menu with stop, force stop, resume, steer, archive
and transfer, the transfer card with its preflight, the remote-access banner,
the status button, and the workbench frame's own lines (what it says with
nothing selected, while reading, when a page is drawn against a conversation
that has moved).

What changes is words: casing of the vocabulary, Title Case on pane and
section headings, the verb on every button, empty states that say what fills
them, progress lines that name what is being read, explanations cut to a
sentence or two with anything longer behind a disclosure, ellipses as the one
character, no em-dashes. Where a long explanation carries something the human
genuinely needs, keep it behind a disclosure rather than deleting it.

Nothing behaves differently. The web suite asserts on copy by text and by
accessible name, so every assertion that names a changed string moves with it,
and the suite is green at the end. Where a string is shared with another
surface through a common component, leave that component to task 05.

## Acceptance criteria

- [ ] Every string on the conversations list, the compose page and composer, the setup row, the adoption and take-up cards, the actions menu and the transfer card follows `docs/copy.md`.
- [ ] Pane and section headings on those screens are Title Case; everything else is sentence case.
- [ ] The web suite passes with its text and name assertions updated, and no behaviour changed.
