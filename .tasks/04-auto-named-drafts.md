# 04. Auto-named drafts

## What to build

Record whose the branch name is. The random name is still generated and stored
when a draft is created, but it is Verkstead's own until the human types one —
a distinction the record keeps, not something read off the name's shape.

While the name is Verkstead's, the draft's branch field renders empty with the
placeholder **Automatically select**, and the draft is titled **Draft** — in
the sidebar row, the pane header, and the row's read-aloud label alike. The
random name appears nowhere in the UI.

A typed name becomes the human's: it is validated and renameable while
drafting exactly as today, and it is the title at once. Clearing the field
back to empty hands the name back to Verkstead — the stored random name
stands again and the title returns to "Draft". Two drafts on one Repo reading
"Draft" beside the same Repo name is accepted as it is; drafts are few and
short-lived.

This task is display and record only: what the title does after grilling
starts, and the instruction that renames the branch, are tasks 05 and 06.

## Acceptance criteria

- [ ] A new draft shows the empty branch field with the placeholder and is
      titled "Draft"; the random name is drawn nowhere while the name is
      Verkstead's.
- [ ] A typed name is recorded as the human's, shown as the title at once,
      and behaves exactly as today; clearing the field returns the name to
      Verkstead and the title to "Draft".
- [ ] The sidebar's read-aloud label agrees with the drawn title in both
      cases.
