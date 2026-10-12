# 02. The app shows a notification

## What to build

The main process holds the notices stream for the app's whole run and shows
each notice as a native Electron notification. It opens the stream on the
loopback once the sidecar is healthy, logged in with the Workbench Key read
from the Data Directory the way the window's link is built — read afresh, and
read again on a 401, which is the key having been reset from the phone — and
reconnects after a drop with a short, bounded wait. A notice that arrives while
the stream is down is lost, as a push is when its service is; nothing is
replayed.

Each notice draws the sentence as the title, the repo as the body, with the
platform's own sound — and only while the window is not in front: hidden to
the tray, minimised, behind another app, or never shown on a login start. A
focused window already draws the news. A click brings the window forward the
three steps the tray's Open takes. The app logs that it is holding the stream,
in a line the release legs can read, and logs each notice it shows and each it
withholds.

What is pure is kept out of the edge: parsing the stream's frames into
notices, and deciding whether a notice is worth showing given the window's
state and the setting to come in task 04, both under vitest.

## Acceptance criteria

- [ ] With the window hidden to the tray, a Set asked of the sidecar raises an
      OS notification titled with the Set's title and bodied with its project
- [ ] With the window focused, the same Set raises none, and the log says it
      was withheld
- [ ] A click on the notification brings the hidden window back on screen
- [ ] A key reset from another browser is followed by the stream being held
      again under the new key
- [ ] The stream line is in the log, and the frame parser and the showing
      rule have vitest coverage
