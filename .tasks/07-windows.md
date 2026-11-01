# 07. Windows

## What to build

On Windows a session runs as Verkstead's session account, a separate user, and
agy's keyring there is the Credential Manager, reachable by that account and
per user — so nothing refuses it, and every agy Profile on one device would
share one vault. The design's rule is that a login is the Profile's, not the
device's. Decide by measurement on winvm, in this order:

1. Whether agy can be made to use its file store on Windows at all — an
   environment it reads (its WSL detection is file-based), a setting, a
   missing vault. If it can, the Linux design stands unchanged: the login runs
   as the session account into the Profile's root (ADR-0022 already starts a
   Windows login that way, behind a boundary of its own), the file is linked
   and written back, one login per Profile.
2. If it cannot, the login lands in the session account's vault: run it as that
   account, record that this platform has one agy login per device rather than
   per Profile, say so on the Profile card where the file would have been, and
   make the signed-out check read the vault's state the way the others read
   the file's.

Either way a session then runs, asks, is judged idle and resumes on winvm, with
the Windows grant following the allowlist as it does for every harness: an
entry on each built directory, one on each joined entry, one on a linked login.

## Acceptance criteria

- [ ] The measurement is recorded in ADR-0011's amendment with the agy version and the Windows build it was made on, and the choice it forced.
- [ ] On winvm an agy Profile signs in by the card and a session launches, asks and resumes under it.
- [ ] The Windows grant covers the root, the joined entries and the login, and no file of the human's own account is written by a session.
