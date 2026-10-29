# 02. The offer, on a Mac

## What to build

The same Install Offer on the platform Electron does most of for us. ADR-0020,
*Amended: a download offers to install itself*, is the decision record.

**The decision** gains its Mac arm: packaged, `darwin`, and Electron's own
`isInApplicationsFolder()` answering false — which it does from the dmg, from
Downloads, from the Desktop, and from Gatekeeper's translocated path. The
hidden-start and don't-ask readings are the shared ones from task 01. Because
the Electron call cannot be made from a module vitest runs, the edge hands the
decision its answer as a value, the way `app.isPackaged` is handed in today.

**The dialog** is the same native message box with the Mac words the ADR gives
it: *Verkstead is running from outside Applications. Installing moves it into
Applications and opens it from there.*

**Install** is `app.moveToApplicationsFolder()` into `/Applications`, which
moves the bundle, trashes a copy that was there, asks for the administrator's
password where the folder wants one, and quits and relaunches on its own. Its
conflict handler lets the default stand for now — task 03 puts the version
reading in front of it. A move that returns false, which is the password
dialog cancelled, and one that throws are both the failure dialog, and the
launch carries on from where it is. The dmg is left mounted. `--install` does
the same with nobody asked.

**Proof on a real Mac**, on the `macos-pack-probe` branch's throwaway workflow
on `macos-15` (see `docs/roadmaps/cluster-updates/02-a-signed-mac-release.md`
for how that branch is used): the packed app run out of the mounted dmg with
`--install` relaunches from `/Applications`, the relaunched process's
executable path is under `/Applications` and contains no `AppTranslocation`,
and the app comes up serving. The result is written into this task's commit
message and, if the move behaves otherwise, into a memo.

The README's Mac line changes from dragging the bundle into Applications to
opening it and pressing Install, the Gatekeeper sentence kept.

## Acceptance criteria

- [ ] Run from the dmg or from Downloads, the dialog shows; Install moves the bundle into `/Applications` and the app comes back running from there; Skip and the box behave as on Linux.
- [ ] On the probe runner, the relaunched app's path is under `/Applications`, not translocated, and the app serves; a cancelled or failed move shows the failure dialog and the app carries on.
- [ ] The Mac arm's decision and words are under vitest on Linux, as the login-item arm's are, and `desktop/`'s three checks are green.
