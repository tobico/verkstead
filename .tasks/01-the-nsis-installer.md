# 01. The NSIS installer, installed on the runner

## What to build

Replace the msi with electron-builder's `nsis` target, and prove the result
the only place it can be proven: on the `desktop-windows` leg of the release
workflow.

**The installer.** It is per-user, one-click and asks for no admin. It is
x64 only and unsigned, as today. It is published as `Verkstead-x86_64.exe`.
It creates a Start-menu entry and no desktop shortcut, and it starts the app
after a double-click install, as the msi did. An NSIS include script
(electron-builder's `include` key) does the two things no configuration key
can:

- **The install directory is `%LOCALAPPDATA%\Programs\Verkstead`.** With
  nothing said, electron-builder's per-user one-click install uses the package
  name, `verkstead-desktop`: see the memo *NSIS installs to verkstead-desktop
  by default*. A `preInit` that writes `InstallLocation` under the install
  registry key fixes it. The directory must not change, because the Run value
  Launch on Startup wrote names `Verkstead.exe` in it, and that value has to
  keep working with no code change.
- **`resources\cli` goes on the user PATH and comes off on uninstall.** Under
  `HKCU\Environment`, appended, never duplicated, never the install root
  (the root holds `Verkstead.exe`, and Windows looks names up without regard to
  case, ADR-0020). It broadcasts the environment change so that a terminal
  opened afterwards sees it.

The Start-menu entry has to carry the AppUserModelID `net.tobico.Verkstead`,
which electron-builder's NSIS template sets from `appId`. The toast check
depends on it, and so does `main.ts`'s `setAppUserModelId`.

**The leg.** It packs the exe and installs it silently (`/S`, through
`Start-Process -Wait`). It re-aims the checks the leg already makes at the NSIS
install:

- the files are in the profile, and the install is registered under
  `HKCU`'s Uninstall key and not `HKLM` (the msi's `Installer\Products` check
  does not apply)
- the Start-menu entry opens the launcher, which is a windowed program
- a fresh terminal finds `resources\cli\verkstead.exe`, and the root is not on
  PATH
- the window, the tray icon and the viewer come up
- a notice is shown as a toast

It then uninstalls with the uninstaller's `/S`. It uploads the exe in the msi's
place, and the `publish` job's list of desktop asset names says
`Verkstead-x86_64.exe`.

**Gone:** the `msi` target and section, `msiProjectCreated`,
`desktop/scripts/msi.mjs`, and the msi upgrade test. Keep the UpgradeCode
string somewhere task 02 can find it: an NSIS define in the include script, or
a comment.

**Proof.** The release workflow runs from `main` only, and it bumps the
version, tags and publishes. So prove the leg on a **throwaway probe branch**:
the stage branch's tree plus a push-triggered copy of the leg, never merged,
and deleted afterwards. The human approved this (Set 1393 Q5). The leg needs
the Windows CLI: build it on the runner, as the Mac probe did, because a
released one is too old.

## Acceptance criteria

- [ ] A probe run of the leg passes: the exe installs silently, every check
      above passes, and it uninstalls
- [ ] After the install, a fresh terminal finds `…\Programs\Verkstead\resources\cli\verkstead.exe`;
      the install root is not on the user PATH
- [ ] After the uninstall, the PATH entry, the install directory and the
      Start-menu entry are gone; the Data Directory is left alone
- [ ] `publish` expects `Verkstead-x86_64.exe` and no msi; nothing in
      `desktop/` packs or names an msi target
