# 03. The Windows installer

## Goal

The release ships an NSIS installer for Windows in place of the msi: a
per-user install with no admin, to the same directory the msi used, with the
CLI's directory on the user PATH and the Run-key start untouched. The
workflow's Windows leg installs it silently on the runner, checks the PATH
entry and that the app starts, and uninstalls it; the msi, its configuration
and its upgrade-over-itself test are gone. A machine holding the msi installs
the first NSIS release over it by hand once, and that release's notes say so
in one line. The README's Windows section and `docs/releasing.md` follow.

## Decisions in force

- **NSIS replaces the msi, because electron-updater updates NSIS installs
  only** ([ADR-0027](../../adr/0027-cluster-updates.md), *The release*).
  The msi was kept over NSIS in ADR-0020; asked again with the condition
  known, the human let it go with *no reason for the msi*. Shipping both was
  not taken.
- **What NSIS has to keep**: per-user, no admin, the CLI on the user PATH,
  and the install directory the Run-key value and the PATH entry already
  name, so a machine that had the msi starts the same way after.
- **The msi is not upgraded by an NSIS installer.** The one-time by-hand
  install over it is accepted; whether the NSIS installer removes the msi
  product by its UpgradeCode, or the human does, is this stage's grilling to
  settle.
- **Unsigned, as today.** Windows signing was declined and is not
  re-proposed here; electron-updater's own download carries no mark of the
  web, so SmartScreen does not meet the update.
- **The Annex is unaffected.** winvm runs no Annex, and the Keeper's re-pin
  reads the Keeper's version, not its installer.

## Proposed tasks (provisional)

1. **The NSIS target** — the `nsis` target with per-user install, the
   directory, and an include script adding the CLI's directory to the user
   PATH and removing it on uninstall.
   - Installed on a clean user, `verkstead --version` answers from a new
     shell.
2. **The Windows leg** — the workflow builds the installer, installs it
   silently, checks PATH and that the app starts and quits, uninstalls, and
   publishes `Verkstead-x86_64.exe` in the msi's place; the msi
   configuration and its upgrade test removed.
   - The release holds no msi.
3. **The words** — the README's Windows section, `docs/releasing.md`'s
   Windows leg and checklist, the site's download link, and the release
   note line about installing over the msi.

## Re-verify at start

- Assumes the Windows configuration is the `win:` and `msi:` sections of
  `desktop/electron-builder.yml` and the leg is the `desktop-windows` job of
  `.github/workflows/release.yml`, with the msi upgrade test inside it, and
  that the install directory is `%LOCALAPPDATA%\Programs\Verkstead` with
  `resources\cli` on the user PATH.
- Assumes the Run-key start is `desktop/src/startup.ts`, `runkey.ts` and
  `registry.ts`, naming the installed executable's path.
- Assumes `site/index.html` links the msi by asset name and
  `docs/releasing.md` describes the msi's per-user install and UpgradeCode.
- Assumes the `cfg(windows)` suites run here under mingw and Wine, and the
  installer itself is proven only on the runner.
