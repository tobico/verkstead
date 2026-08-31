# 05. The pairing listbox

## What to build

A custom listbox replaces the native `<select>` where a row deserves its mark:
the four pairing pickers (three on the setup card, one on the steer modal) and
the profile form's harness-type picker. Every row draws its harness mark beside
the task-02 reading; the closed control draws the chosen row the same way. The
"No grilling" / "No review" rows carry no mark. Every other picker in the app
stays a native select.

The existing picker component earned its guarantees and the listbox keeps every
one of them — they are the module's whole point, documented at its head:

- What is shown and what would be sent are the same string, always, however the
  option list is rebuilt underneath the choice.
- A chosen row that is no longer among the options falls to the placeholder and
  says so through the same optional `gone` callback; nothing unpicks itself.
- A caller-supplied row that sends the empty string is a choice, not a
  placeholder; "Not chosen" is drawn only when nothing is chosen and no such
  row exists.
- The disabled state, and the label-by-id contract (`<label for=…>` reaches it).

It is operable the way a native one is: full keyboard driving (open, arrows,
Home/End, Enter picks, Escape closes), the ARIA combobox/listbox roles wired so
a screen reader announces rows and selection, and tap-friendly rows on a phone
— the workbench is answered from one, which is what the native picker was kept
for until now.

The list stays **flat** — one row per profile-and-model combination, no
grouping headers — because that is the Pairing's settled shape.

CONTEXT.md moves with the change: the Pairing entry's wording of how a row
reads (`profile — model`) is updated to the new reading, and anything else in
that file the new format contradicts.

## Acceptance criteria

- [ ] All five converted pickers draw mark + reading per row and on the closed
      control; picking, keyboard driving and screen-reader announcement work;
      everything sent over the wire is unchanged.
- [ ] The divergence guarantees hold under test: options rebuilt with the
      choice gone fall to the placeholder and fire `gone`; shown and sent never
      differ.
- [ ] The No-grilling and No-review rows draw text only; all other pickers in
      the app remain native selects.
- [ ] CONTEXT.md describes the new reading.
