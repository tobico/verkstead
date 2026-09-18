# 02. The desktop's log file

## What to build

Two things about the one file the tray's **View Logs** opens.

**The workbench key comes out of it.** ADR-0015 settled that where the login
link is handed out is a fact about the install: the tray's **Open** for the
desktop app, the startup line for the daemon. The desktop app was doing both —
it opens a browser on the link *and* logs the same line the daemon does, key and
all, into a file a menu item opens and anybody reading over a shoulder can see.
So the listening line carries the link where the caller has no other way to hand
one over, and names the address alone where it has. `verkstead serve` is
unchanged: a headless host has nothing else, and a key redacted there would
leave a machine started from a unit file with no way in at all.

The desktop is already the one caller that arrives holding the key before there
is a server to ask one of, so the same handing-over is where it says it hands the
link out itself. The line's own comment today argues the opposite — *the secret
is in the log, and that is the point of it* — and that reasoning is the daemon's
rather than every caller's; it stays, said as the daemon's.

Rejected and not to be reopened: a `workbench-url` verb, which ADR-0015 already
rejected as a second place to print one line; and redacting everywhere.

**The Windows suite asserts the file is unreadable from inside a session**,
beside its existing attempt on the human's own Documents and the account's own
skills. That is the claim worth having whatever the line says: the log lives in
the human's own local application data, a session runs as a local account of
Verkstead's own, and the boundary is what makes a key in a log the human's
problem alone. Attempted rather than read off the access-control list, which is
that suite's rule.

**And the file opens with a byte-order mark**, on a fresh file and after each
roll. Verkstead's own messages are written with em-dashes in them, and the
viewers Windows opens a `.log` in read a file with no mark in the machine's code
page, so the reporter's log was mojibake. The mark belongs to a file being
started rather than to every write: a run appending to a file that already has
content adds none. Rejected: taking the em-dashes out of the messages.

## Acceptance criteria

- [ ] The desktop app's log carries the listening line with the address and no
      `?key=` on it; `verkstead serve`'s stdout still carries the whole login
      link, and the tray's **Open** still opens a browser on it.
- [ ] A fresh log file and a rolled one each open with the byte-order mark, and a
      run appending to an existing file adds none — with tests beside the rolling
      tests in the desktop crate.
- [ ] A session attempting to open the desktop's log file is refused, asserted in
      the Windows suite beside the human's own Documents.
- [ ] ADR-0015 says which install's startup line carries the key and which does
      not, and CONTEXT.md's Workbench Key entry agrees with it.
