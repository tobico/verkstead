# 05. The platform legs and the docs

## What to build

Proof on the three packed apps, and the words. Each release leg already
launches the packed app and reads its log for the window and the tray; each
now also asserts the line that says the notices stream is held. The Linux leg
goes further and proves a notification is delivered under a notification
daemon on the runner, since Electron loads libnotify at runtime there and an
AppImage that cannot find it shows nothing. The Mac leg proves the ad-hoc
signed bundle is accepted by the notification centre, which is where an
unsigned app can fail; the Windows leg proves a toast under the AppUserModelID
the app already sets.

The development guide's desktop section says how to see a notification from a
checkout here — the stream, the focus rule, and the daemon a headless run needs
— beside what it already says about the window and the tray.

## Acceptance criteria

- [ ] All three release legs assert the stream line in the packed app's log
      and are green
- [ ] The Linux leg sees a notification delivered to a daemon on the runner,
      or the AppImage's missing dependency is named and fixed
- [ ] The Mac and Windows legs each show one notification from the packed app,
      recorded in the leg's own log
- [ ] The development guide's desktop section covers notifications
