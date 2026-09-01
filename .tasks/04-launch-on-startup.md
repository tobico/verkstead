# 04. Launch on Startup

## What to build

The last menu item: a checkbox that says whether Verkstead comes up when the
machine's desktop session does. On Linux that is an XDG autostart entry — a
`.desktop` file under the user's autostart directory, named for the app id
`net.tobico.Verkstead` and pointing at the running executable. Checking the box
writes it, unchecking removes it.

**The platform registration is the source of truth and nothing duplicates it.**
The checkbox reads its state from the file itself on every launch, not from a
setting of Verkstead's own: a human who deletes the entry with their desktop's
own tools has unchecked the box, and Verkstead should agree with them rather
than argue. There is no entry in either settings file for this, and none is
added.

**While the box is checked, every launch rewrites the entry** with the path of
the executable that is running. A binary that was moved — downloaded again into
a different directory, or an AppImage put somewhere else — then heals its own
autostart entry the next time it is launched by hand, rather than leaving a
stale entry that silently stops working. That is the decision that makes this
"re-register on every launch" rather than "write once".

**A launched-at-startup Verkstead is an ordinary launch.** It opens the viewer
like any other unless `--no-open` says otherwise, and what the autostart entry
says about that is part of writing it: coming up with a browser window on every
login is the kind of thing that gets the box unchecked. Decide it, and say so
in the entry.

Stages 04 and 05 write the same checkbox against the Windows Run key and a
macOS launch agent. Keep the platform half separable so that those stages add
an arm rather than rework the item.

## Acceptance criteria

- [ ] Checking the box writes an autostart entry naming the running executable
      and the app id; unchecking removes it, and neither writes anything to a
      settings file.
- [ ] The checkbox's state on launch is read from the entry itself, so an entry
      removed outside Verkstead comes back as an unchecked box.
- [ ] Launching a moved executable while the box is checked rewrites the
      entry's path, and a startup launch behaves like any other launch of the
      app.
