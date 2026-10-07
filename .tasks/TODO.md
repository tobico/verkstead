# Claude login modal

A Claude session whose login expires or is refused today needs a human at its
terminal to run `/login` and paste the code back — which is unreliable through
the browser terminal. This lets Verkstead run the login itself: `claude auth
login --claudeai` in the Profile's Built Root, over plain pipes, with the URL
and the code in a modal any device can open. A session that signs out stops as
"Signed out" with a Log in press, a launch under a Profile with no login is
refused before it starts, and a successful login resumes every run it unblocks.

Claude only, and only Profiles at home on this device; other harnesses and
logging in a mirrored Profile across the cluster are left for later.

## Tasks

- [x] 01: Log in from the Profile card — [details](01-log-in-from-the-profile-card.md)
- [x] 02: A rejected code starts again — [details](02-a-rejected-code-starts-again.md)
- [x] 03: Signed-out stop — [details](03-signed-out-stop.md)
- [x] 04: A login resumes the runs it unblocks — [details](04-a-login-resumes-the-runs-it-unblocks.md)
- [x] 05: A late sign-out resumes by itself — [details](05-a-late-sign-out-resumes-by-itself.md)
- [x] 06: Check before launch — [details](06-check-before-launch.md)
- [x] 07: Log in on Windows — [details](07-log-in-on-windows.md)
