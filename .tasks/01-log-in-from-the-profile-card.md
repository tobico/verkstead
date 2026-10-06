# 01. Log in from the Profile card

## What to build

A **Log in** button on the card of a Claude Profile at home on this device
opens a modal. Verkstead runs `claude auth login --claudeai` for that Profile
inside the sandbox, with the Profile's Built Root — the same root a session
gets — so the existing write-back saves the login to the account on Linux, Mac
and Windows alike. `BROWSER=/bin/true` stops it opening a browser on the server.

It runs over plain pipes, not a terminal: claude 2.1.283 prints
`If the browser didn't open, visit: <url>` on one line and then reads the code
from stdin after `Paste code here if prompted >`. The modal shows the URL as a
link with a copy button, a box for the code, and a submit button. The code is
written to the process's stdin; a clean exit, confirmed by `claude auth status`,
is success and the modal says so.

One login per Profile at a time: a second device opening the modal joins the
login already running and sees the same URL. Closing the modal (by the last
viewer) or 10 minutes passing kills the process.

Mirrored Profiles get no button. Write the ADR for the feature (0022) covering
the decisions from planning: separate login on the account rather than typing
into the session, Claude and local Profiles only, the built-root launch, the
phrase rule, resuming on success.

## Acceptance criteria

- [ ] With a fake `claude` that prints the URL and reads a code, submitting the
      code in the modal leaves the login in the Profile's account
- [ ] Two viewers of one Profile see the same URL; there is never a second
      login process for that Profile
- [ ] Closing the modal or the 10-minute limit kills the process
- [ ] No Log in button on a mirrored Profile or a non-Claude Profile
- [ ] ADR 0022 written
