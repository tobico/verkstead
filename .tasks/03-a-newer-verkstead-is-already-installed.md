# 03. A newer Verkstead is already installed

## What to build

Before either platform's Install replaces what is in Applications, the installed
copy's version is read and compared with the running one, as the ADR settles.

**Reading it** is a pure function of text on both platforms. A Mac's is
`CFBundleShortVersionString` out of `/Applications/Verkstead.app/Contents/
Info.plist`, read the way `launchd.ts` reads two keys out of a plist. Linux's is
`X-AppImage-Version` out of the desktop entry electron-builder writes into the
image's root — `net.tobico.Verkstead.desktop` — which the AppImage runtime
extracts with `--appimage-extract` of that one file into a temporary directory
of the app's own, without starting Electron, so every release ever shipped
answers. An installed file that is not there, or whose version cannot be read,
is treated as older: the one being run is what the human has in hand.

**Comparing** is semver, a pre-release older than its release, equal counting
as not newer.

**Newer**: *Open the installed Verkstead?* over *Verkstead 0.4.0 is already
installed, and this is 0.3.0.*, with **Open installed** relaunching the
installed copy through Electron's relaunch with its path as the executable and
ending this one, and **Keep this one** carrying on with the launch as it is.
**Older or equal**: the Install goes ahead and replaces it — on a Mac through
the conflict handler returning true, on Linux by renaming over the file.

An installed copy that is running never reaches this: the single-instance lock
has handed the launch over before any dialog. `--install` follows the same
reading and opens the installed copy when it is newer, saying so in the log.

## Acceptance criteria

- [ ] With an older or equal Verkstead in Applications, Install replaces it with no second dialog, on both platforms.
- [ ] With a newer one there, the open-installed dialog shows; Open installed starts it and ends this one, and Keep this one carries on from where it is.
- [ ] Both readers, the comparison and the dialog's words are under vitest, with a pre-release older than its release and an unreadable version treated as older.
