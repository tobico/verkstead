# 05. The wizard, the Set sheet, the share page and the shared pieces

## What to build

Read `docs/copy.md` first. Then bring the rest of the viewer into line with
it: the setup wizard and each of its steps (git author, dependencies,
accounts), the Set sheet — its contents, standing lines, answering, the
postscript and the diff — and the share page; and the components every page
draws through: modals, toasts, the notices (nothing here, something went
wrong, the note), joining and attaching, the path field, the picking and
truncation pieces, the copy button, and whatever strings the app shell itself
carries.

The Set sheet is where the human reads on a phone, and it is the surface the
*question set* rule is for: *question set* in full wherever it stands alone,
*set* only where *question* is already in the sentence, and never a stray
capitalised *Set*. Its standing lines (answered already, locked, gone, closed)
are refusals in the guide's sense and take that shape. The wizard's step
headings go Title Case like every other heading.

Nothing behaves differently, and the web suite's assertions move with the
strings.

## Acceptance criteria

- [ ] Every string in the setup wizard, the Set sheet, the share page and the shared components follows `docs/copy.md`.
- [ ] The Set sheet reads *question set* in full and carries no capitalised *Set*; its standing lines are full sentences.
- [ ] The web suite passes with its assertions updated, and no behaviour changed.
