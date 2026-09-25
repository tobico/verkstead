# 03. The bridge

## What to build

A preload bridge, which is how the page reaches the app: it exposes **the
platform** the app is running on, **the desktop settings** — read and write, over
the file task 02 made — and **opening the log file**, which is the same act the
tray's **View Logs** performs. The main process enacts each of the three
(ADR-0020, Set 847 Q12).

**A set takes effect in that run, not at the next launch.** Turning the tray off
over the bridge takes the icon away and the close policy falls to Quit
immediately; turning it back on puts the icon back. This is what makes the page
in task 04 a control rather than a form somebody has to restart the app to see
the effect of.

**The bridge is the app's window's alone.** The same document served to a browser
on this machine, or to a phone over the tailnet, carries none of it — so a page
has to read its absence as *this is not the app* rather than fail on it. That is
the whole mechanism by which a phone never sees the Desktop page, and nothing new
goes on the wire to achieve it.

**Main validates what arrives.** A renderer is a renderer: a setting coming up
the bridge is checked against the shape before it reaches the file, and a value
that is not one of the positions or not a boolean is refused rather than written.

**One build constraint worth knowing before starting**, because it decides how
the file is written: a preload script ignores the package's `"type": "module"`,
so an ESM preload has to be a `.mjs` *and* must not be sandboxed, while a
sandboxed preload runs as plain CommonJS. The project compiles rather than
bundles, so whichever is chosen, what the window's preferences name has to be
what the build actually emits.

**The wall.** The preload imports `electron`, so it joins the short list in the
lint configuration of files allowed to, with a line saying why it is there — the
list is a list rather than a directory so that adding to it is deliberate. What
the bridge *decides* — the shape it exposes, and the validation above — stays in
modules vitest can run.

## Acceptance criteria

- [ ] A set made over the bridge is on the JSON file and is still there after a
      restart.
- [ ] Turning the tray off over the bridge takes the icon away in that run and
      turning it on puts it back, and the close policy reads the new value
      without a restart.
- [ ] The same page opened in a browser on this machine has no bridge, throws
      nothing, and reads as a machine with no app; a setting of the wrong shape
      sent up the bridge is refused rather than written.
