# 03. The two marks

## What to build

Two marks on the version line task 01 draws, both:

- **Differs from this device:** a row whose version is not this device's draws
  the version line in a colour of its own (a token on `:root` with its dark
  pair, as the viewer's other colours are), with **no word** beside it — the
  answer to wording Set 1381. A row with no version has nothing to colour.
  Compared in the viewer against `DevicesView.this.version`.
- **A newer release exists:** a row whose version is older than the latest
  stable release carries the tag **update available** (lower case, Set 1394),
  drawn where and how the *unreachable* tag is. This device's row included.

The newer-release verdict is the Update Notice's own daily check — stable
releases only, in memory — with no second check. **But today that check throws
the number away**: its verdict is `Current` whenever this device is up to date,
so a member behind an up-to-date device could never be tagged. So the check
keeps the latest stable release it found whatever it concludes about this
device, and `/api/ui/update` still answers exactly what it did. The server
compares each row's version against it as semver (it already parses with
`semver`) and the Devices view carries the verdict per row, so the viewer does
no version arithmetic. With the check turned off or not yet answered there is
no latest release, and no row is tagged.

## Acceptance criteria

- [ ] A member a release behind this device shows its number in the differing
      colour and no word; a member on this device's version is drawn plainly.
- [ ] With the check answered with a newer release, every row older than it,
      this device's included, carries *update available*; a row on that
      release, or with no version, does not.
- [ ] With the check off or unanswered, only the differing colour can appear.
- [ ] The Update Notice banner behaves exactly as before.
