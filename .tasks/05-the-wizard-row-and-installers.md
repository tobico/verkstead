# 05. The wizard row and the installers

## What to build

A row for the Antigravity CLI in the onboarding wizard beside the four
harnesses, found where a session would run it, and installed by the vendor's
own installer where it is ticked:

- Linux and a Mac: `curl -fsSL https://antigravity.google/cli/install.sh |
  bash`, one file at `~/.local/bin/agy`, no versions directory, no symlink;
  the script edits no shell profile, and `agy install` after it may. The
  directory is `.local/bin`, which the session floor already reaches.
- Windows: `irm https://antigravity.google/cli/install.ps1 | iex`, landing
  `%LOCALAPPDATA%\agy\bin\agy.exe`, per user, the PATH not modified by the
  script; what a ticked row writes to the session path is that directory.

Both download from the vendor's manifest service and verify a SHA-512, both
refuse to overwrite an existing `agy` (it self-updates in the background). The
wizard's instruction tabs show the same lines for the machine Verkstead cannot
install on. No distribution package, Homebrew formula or npm package exists for
it, so there is no package arm.

## Acceptance criteria

- [ ] The wizard draws an Antigravity CLI row, found or missing, and the instruction tabs show the three platforms' lines.
- [ ] A ticked row on Linux installs `agy` into `~/.local/bin` and the next session finds it on its `PATH`.
- [ ] The Windows and Mac units carry the right line and landing directory, proved by the unit tests the other vendors' installs have.
