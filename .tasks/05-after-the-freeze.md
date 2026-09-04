# 05. After the freeze

## What to build

Once the Brief is frozen the files stay on the record where the human goes to
read what the work was opened on. The frozen Brief pane draws the attached
files as a read-only row under the Brief, above the Configuration: the names,
each with its size, no remove press. Nothing on the Timeline card changes.

A Share carries the same row and never the bytes: the names and sizes travel
with the record into the file, and the attachments directory is not read on
the way out. The Share viewer draws the row on the Brief pane exactly as the
workbench does.

Where an origin other than the Brief exists later the row is the Brief's own
files; this task draws the origin it has.

## Acceptance criteria

- [ ] A grilling Conversation's Brief pane lists its attachments by name and
      size with no remove press, and the composer no longer draws the paperclip.
- [ ] A downloaded Share shows the same names on its Brief pane and contains no
      attachment bytes.
- [ ] A Conversation with no attachments draws no row at all.
