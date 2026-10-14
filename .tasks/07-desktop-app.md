# 07. The desktop app

## What to build

Read `docs/copy.md` first. Then bring the desktop app's few strings into line
with it: the quit confirmation, the tray and application menus, the notices it
raises (no log file to open, the sidecar not answering, a port already taken,
a mount gone), the loading page, and the window titles.

The quit dialog is already the guide's shape — a question naming the act, one
sentence of consequence, the verb against Cancel — and is the example to hold
the rest to. Menu items are sentence case, with the ellipsis on any that open
a further step. A notice is a full sentence stating what is true and then the
one action where there is one.

Nothing behaves differently, and the desktop suite's assertions move with the
strings.

## Acceptance criteria

- [ ] Every string in the desktop app's dialogs, menus, notices, loading page and window titles follows `docs/copy.md`.
- [ ] The desktop suite passes with its assertions updated, and no behaviour changed.
