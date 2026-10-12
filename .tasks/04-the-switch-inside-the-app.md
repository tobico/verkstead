# 04. The switch inside the app, on by default

## What to build

A third Desktop Setting, `notifications`, kept in the app's own settings file
beside the close policy and the tray icon, default on — so a fresh install is
told with nothing pressed. A set is enacted in the run it is made in: off stops
the next notice at once, on starts showing them again, with the stream held
either way.

Inside the app the switch on the Settings head is this setting, read and
written over the bridge, and no browser permission or push subscription is
involved; the same switch in a browser on this machine and on a phone is the
Web Push switch it always was. The line under the switch says where the app
cannot show one — a desktop without notification support — and nothing where
it can; a refusal in the OS's own settings is the OS's to lift and is not
guessed at here. The Desktop section gains no second control.

## Acceptance criteria

- [ ] A fresh user data directory notifies with nothing pressed, and the
      settings answer carries `notifications: true`
- [ ] Flipping the switch off inside the app withholds the next notice in the
      same run; flipping it on shows the one after
- [ ] A set carrying anything but a boolean is refused and the switch goes
      back where it was
- [ ] The page's switch test drives both arms: the bridge present, and absent
      with the browser's push untouched
- [ ] CONTEXT.md's *Desktop Settings* names the third setting and what the
      head's switch means inside the app
