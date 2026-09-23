# 07. Saving

## What to build

Saving is explicit, and a write over a file that has moved is refused
(ADR-0019, *Versioned reads, and a stale write is refused*).

**A dot on a dirty tab, and Ctrl+S writes it back.** The write names the
version the read handed over. Closing a dirty tab confirms first, which is the
other half of the × rule task 02 built for terminals.

**A write over a version that has moved is refused in the body**, beside the
other refusals the files API answers with, and it is a sentence rather than a
status code. What the pane draws over that refusal is a bar offering **Reload**
or **Keep mine**: Reload takes the disk's text and the version with it, and
Keep mine keeps the human's text and the version the disk now has, so that the
next Ctrl+S lands. Stage 04 of the roadmap makes the same bar appear the moment
the disk moves rather than at the next save; here it is drawn by the refusal.

**A read-only root takes no write at all**, and the refusal says so.

Last writer wins is not what this is: the collision is the point, and an
agent's edit silently overwritten by a human who never saw it is exactly what
the version exists to surface.

## Acceptance criteria

- [ ] A dirty tab shows a dot, Ctrl+S writes the file to disk, and the dot
      goes.
- [ ] Closing a dirty tab confirms first.
- [ ] A file changed on disk between the read and the save is refused and the
      Reload / Keep mine bar appears; **Keep mine** then saves, and **Reload**
      takes the disk's text.
- [ ] A save into a read-only root is refused with its own sentence.
