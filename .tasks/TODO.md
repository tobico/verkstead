# The Windows installer

The release ships an NSIS installer for Windows in place of the msi, because
electron-updater updates NSIS installs and not msi ones (ADR-0027). It is a
per-user install with no admin. It goes to the directory the msi used,
`%LOCALAPPDATA%\Programs\Verkstead`, and puts `resources\cli` on the user PATH,
so the Run value and the PATH entry a machine already has still name the same
paths. The workflow's Windows leg installs the exe silently on the runner,
checks it, and uninstalls it. The msi, its configuration and its upgrade test
are gone.

A machine holding the msi installs the first NSIS release over it by hand once:
the installer finds the msi by its UpgradeCode and removes it first, and that
release's notes say so in one line. The README, `docs/releasing.md`,
`docs/development.md` and the site follow.

Roadmap stage: [03: The Windows installer](docs/roadmaps/cluster-updates/03-the-windows-installer.md)

## Tasks

- [x] 01: The NSIS installer, installed on the runner — [details](01-the-nsis-installer.md)
- [x] 02: Installing over what is already there — [details](02-installing-over-what-is-there.md)
- [x] 03: The words — [details](03-the-words.md)
