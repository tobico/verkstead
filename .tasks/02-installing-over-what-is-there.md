# 02. Installing over what is already there

## What to build

The two installs a real machine makes on top of an existing one, both proven on
the Windows leg.

**The exe over itself.** This is a by-hand reinstall, and it is the shape
electron-updater's update takes later (that update is stage 04's). Installing
the same exe again leaves one row in Installed apps, one PATH entry and one
Start-menu entry. This is the check that the include script's PATH addition
does not duplicate. The human asked to keep this check (Set 1393 Q4), though
the brief retired the msi's version of it.

**The exe over the msi.** An NSIS installer cannot upgrade an msi, and left
alone the two would share one directory: two rows, two PATH entries, and an msi
whose later uninstall deletes the exe's files. The human decided (Set 1393 Q2)
that **the installer removes the msi itself**. Before it extracts anything,
the include script finds any product installed under the msi's UpgradeCode
`{A4727089-F5B9-40A6-A196-1856E8D6827A}` (with `MsiEnumRelatedProducts` through
NSIS `System::Call`), and uninstalls it quietly with `msiexec /x <ProductCode>
/qn`. A per-user msi needs no admin. If the msi is still running, it is stopped
the way the installer already stops a running app. A machine with no msi is not
changed. The Data Directory and settings are not touched, and Launch on
Startup's Run value is not the msi's, so it survives.

**The leg.** After task 01's checks, it installs the exe over itself and counts.
Then it uninstalls, installs the last Release's `Verkstead-x86_64.msi`
(fetched as the old upgrade test did), installs the exe over it, and counts
again. It reads the PATH entry back, so the surviving entry is shown to be
`resources\cli`. Where the last Release has no msi, that half is skipped with a
warning saying it went unchecked, as before. Once a Release ships with no msi,
this half will skip itself every run. Say so in the step's comment, and say
that the check retires along with the removal code.

**The Run value.** With Launch on Startup registered before the install over
the msi, the value under `HKCU\…\Run` still names
`…\Programs\Verkstead\Verkstead.exe` afterwards. Assert that on the leg too,
if registering is reachable there. Otherwise prove it by a reading of the value
in the step's log.

Proven on a throwaway probe branch, as in task 01.

## Acceptance criteria

- [ ] The exe installed over itself leaves one Installed-apps row, one PATH
      entry and one Start-menu entry
- [ ] The exe installed over the last Release's msi leaves one product (the
      msi is gone from `HKCU\Software\Microsoft\Installer\Products`), one PATH
      entry naming `resources\cli`, and one Start-menu entry
- [ ] Installed on a machine with no msi, the installer changes nothing it
      does not own
- [ ] A probe run of the leg passes both halves
