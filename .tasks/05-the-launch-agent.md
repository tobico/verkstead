# 05. The launch agent taken over, and a hidden login start

## What to build

**Launch on Startup on a Mac is Electron's login-item API** (ADR-0020, Set 846
Q9a), and that arm is written and unit-tested already: `startup.ts` reads
`openAtLogin` back with the same arguments it registers, rewrites the
registration at every launch while there is one, and `hidden()` reads a login
start off `atLogin` — the platform saying its login item started this run. What
this task adds is the one thing the API knows nothing about, and then drives the
whole of it on a Mac.

**The tray app's launch agent is taken over, once.** It sits at
`~/Library/LaunchAgents/net.tobico.Verkstead.plist`, and left alone after an
upgrade it is a launch agent firing at every login at a path this roadmap
deletes, while the box on the Desktop page reads off. So the first launch reads
it, carries what it said into the new registration, and removes it — whichever
it said, because a file naming a binary that is gone is worth nothing either
way. Once: the file is gone, so the next launch finds nothing.

**What is read is presence plus two keys**, exactly as
`crates/desktop/src/startup/launchd.rs`'s `says_on` reads them: `Disabled` set
true is off, `RunAtLoad` set false is off, and anything else — the key absent,
the value not a boolean, the file not really a plist — is the file being there,
which is most of what it says. **What the file *names* is not read at all.** It
names `<bundle>/Contents/MacOS/verkstead`, the verb `desktop` and `--no-open`
rather than the launcher script, `current_exe` being the binary the launcher
execs; either way it is a path this roadmap deletes.

**On `darwin`, and only where there is a registration to make.** `startup.ts`
answers `nowhere` on an unpackaged run by decision, so a checkout run that
deleted a developer's plist while offering them no box to tick would be the
worst of both. And it never turns anything off behind anybody's back: an agent
that said on becomes a login item, one that said off becomes nothing, and the
file goes in both cases.

All of it is path, text and the injected `LoginItem`, so every arm is an
ordinary vitest on this Linux runner — the agents directory handed in the way
the autostart directory already is, rather than reached for.

**Then the hidden login start, on a real Mac.** Nothing asks for `openAsHidden`:
the pinned Electron marks it deprecated and says it does nothing on macOS 13 and
up, and `args` on a login item is Windows' alone, so the `--hidden` the other
two registrations carry is not a thing this platform can be told. What answers
instead is `wasOpenedAtLogin`, which is not deprecated, read through
`Startup.atLogin` and handed to `hidden` exactly as the flag is elsewhere. Only
a real log out and back in exercises it.

**And the `activate` handler is what to watch.** It fires when the application
is activated, *including* at a first launch, and `main.ts` answers one that
arrives before the window exists by bringing the window forward once there is
one. A login start macOS does not activate never raises it — but a login start
that shows its window anyway is that handler rather than `hidden`, and the log
is how the two are told apart.

**One thing the pinned Electron has that the arm does not read.** On macOS 13
and up `setLoginItemSettings` is `SMAppService`, and `getLoginItemSettings`
carries a `status` — `not-registered`, `enabled`, `requires-approval` or
`not-found` — beside `openAtLogin`. A human who switches Verkstead off in System
Settings → Login Items leaves a registration in a state `openAtLogin` alone may
not describe honestly. What the box should say then is this task's to look at
and settle; `Registration` already carries a `why` for a line under it.

## Acceptance criteria

- [ ] A home carrying the tray app's plist comes up registered through the
      login-item API with the plist gone and the box reading on; one whose plist
      says `Disabled` or `RunAtLoad=false` comes up unregistered with the plist
      gone; one carrying none is untouched; and an unpackaged run touches no
      plist at all.
- [ ] Every arm of that is a vitest on this Linux runner, and the box on the
      Desktop page reads the registration back on a real Mac — ticked, unticked,
      and after a change made in System Settings.
- [ ] With the box ticked and the menu bar icon on, a real log out and back in
      brings Verkstead up with no window on the screen and its log says the
      launch was a login's; with the icon off, the window comes up.
