# 01. The offer, on Linux

## What to build

The Install Offer, end to end, on the one platform the Linux runner can run it
on. Read ADR-0020, *Amended: a download offers to install itself*, first: it is
the decision record, and this task builds its Linux half.

**The decision**, as a pure module beside `startup.ts` that vitest runs with
values handed in: the offer is made when the app is packaged, the platform is
Linux, `$APPIMAGE` is an absolute path not under `~/Applications` (the home
read by the same rule the autostart directory is read by), this is not a hidden
start (what `hidden()` already answers), and `desktop.json` has not been told
don't ask. A checkout run, a hidden start and an installed copy all answer *no
offer*, each for its own reason, and the log says which.

**The dialog**, in the lifecycle after the single-instance lock and before the
port probe and the sidecar, so nothing has been started when the app hands
over. It is the platform's native message box with its own checkbox, the words
exactly as the ADR has them: *Install Verkstead?* over a sentence naming where
it runs from and what installing does, **Install** and **Skip**, **Don't ask
again**. `app.whenReady()` moves ahead of it if the message box wants a ready
app. Skip carries on with the launch as it is. The box ticked writes a fourth
key into `desktop.json`, read and validated like the three before it, and the
Desktop Settings vocabulary names it.

**Install**: `~/Applications` made if it is not there; `$APPIMAGE` moved to
`~/Applications/Verkstead-x86_64.AppImage`, executable; the desktop entry
written at `~/.local/share/applications/net.tobico.Verkstead.desktop` naming the
copy with the path quoted the way the autostart entry quotes its own, with the
name, the comment, the category and the keywords `electron-builder.yml` gives
the packed entry, `StartupWMClass` set to the app id, and `Icon` naming what the
icons are installed as; the icon set copied out of the mounted image's
`usr/share/icons/hicolor` tree into `~/.local/share/icons/hicolor` under the
same sizes; and the app relaunched from the copy with Electron's relaunch, which
waits for this process to exit. Everything that decides a path or a file's text
is a function of values and is under vitest; only the moving, the writing and
the relaunch are the edge's.

**`--install`** on the command line is the dialog's Install with nobody asked:
outside Applications it installs and relaunches, inside it says so in the log
and carries on.

**Failure** — a move or a write that could not be made — is an error dialog
saying why, by the two the app already shows, and the launch carries on from
where it is.

The README's Linux line changes from making the file executable and running it
to running it and pressing Install.

## Acceptance criteria

- [ ] Run by hand from `~/Downloads`, the dialog shows before the server starts; Install lands the file, the entry and the icons, and the app comes back running from `~/Applications` with its log saying so; the applications menu lists Verkstead with its icon.
- [ ] A `--hidden` start, a checkout run, and a run from `~/Applications` never ask; Skip asks again at the next launch; Don't ask again holds across launches and survives a hand-edit of another key.
- [ ] `--install` from outside Applications installs and relaunches with no dialog, and from inside does nothing but log it.
- [ ] The decision, the entry's text, the icon plan, the key's reading and writing, and the dialog's words are under vitest, with `pnpm lint`, `pnpm typecheck` and `pnpm test` green in `desktop/`.
