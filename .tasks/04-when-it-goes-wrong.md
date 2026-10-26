# 04. When it goes wrong

## What to build

What the human sees when an update fails, at each point it can fail. **No
rollback** (ADR-0027): the dialog, the trouble line, and a higher release.

- **The download or verify fails while the old server is up**: the app posts
  the failure back with the Workbench Key, the row leaves *updating…* for its
  trouble line carrying the updater's reason, and the device stays on its
  version. The Annex row's trouble line is the model.
- **A start that never comes up shows a dialog** (Set 1404, Q3). Today the app
  only logs *the server never came up* and exits 1, with no dialog
  (`desktop/src/main.ts`, after `healthy`). Every such start gets a dialog in
  `docs/copy.md`'s voice, as the app's other refusals have (`missing`,
  `foreign`), not only the starts after an update — then exits as now.
- **The app remembers the version it updated to**, in its own userData beside
  the desktop settings, at the moment it hands over to `quitAndInstall`. A
  later start that comes up on a **different** version than the one remembered
  posts the trouble line (the update did not land — say, the platform's
  installer failed quietly) and forgets it; a start on the remembered version
  just forgets it.
- The proof script from task 03 gains a corrupt-file run: the feed serves an
  AppImage whose sha512 does not match its metadata.

## Acceptance criteria

- [ ] A feed serving a corrupt file leaves *updating…* for a trouble line naming the updater's reason, and the device stays on its version.
- [ ] A server that never answers health shows a dialog, then the app exits.
- [ ] A relaunch that comes up on a version other than the one updated to puts the trouble line on the row.
