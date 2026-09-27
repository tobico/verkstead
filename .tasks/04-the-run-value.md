# 04. The Run value taken over, and a hidden sign-in start

## What to build

**Launch on Startup on Windows is Electron's login-item API** (ADR-0020, Set 846
Q9a), which is the Run key underneath, and that arm is written and unit-tested
already: `startup.ts` answers `{ login: true }` on `win32`, reads `openAtLogin`
back with the same arguments it registers — which is half the question on this
platform, a registration there being a command line — rewrites the registration at
every launch while there is one, and `hidden()` reads a sign-in start off
`--hidden` in the arguments. What this task adds is the one thing the API knows
nothing about, and then drives the whole of it on a real Windows, which nothing has
yet done.

**The tray app's own Run value is taken over, once.** It sits under
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, is named for the app id —
`net.tobico.Verkstead` — and holds `"<exe>" desktop --no-open`, a verb stage 08
removes. Electron's API writes a value of its own and has never heard of that one,
so left alone after an upgrade it is a sign-in start of a binary this roadmap
deletes while the box on the Desktop page reads off. So the first launch reads it,
carries whether it was there into the new registration, and removes it — whichever
it said, because a value naming a binary that is gone is worth nothing either way.
Once: the value is then gone and the next launch finds nothing. This is the Windows
twin of `launchd.ts` and should read like it, down to where it is called from in
`main.ts` — before anything reads the registration, because on a machine that had
that app the value is what the human asked for.

**What is read is presence, and that is the whole of it.** A registry value has no
`Disabled` and no `X-GNOME-Autostart-enabled`: the value is there and Verkstead
started at sign-in, or it is not and Verkstead did not, which is what
`crates/desktop/src/startup/run_key.rs` says for itself. **What it *names* is not
read at all**, the way the plist's command is not: whichever it said, what stands
afterwards is this app registered through the API, or nothing. And **what this
cannot see is the Startup tab** — a human who switched Verkstead off in Task
Manager is recorded in a key of Explorer's own, `StartupApproved`, which is nothing
to do with this value. That is the honest answer the Rust arm already gave and it
carries over unchanged.

**One decision has to be made about the new registration's own value name, and it
is what keeps the take-over a one-shot.** Electron names the value it writes after
the application rather than after the app id. If the two names were the same, the
launch after a take-over would read its *own* registration as the tray app's,
re-register, and delete it — a box that unregisters itself every other launch. So
the value this app writes must not be called `net.tobico.Verkstead`, and the
take-over reads that one name and nothing else. Giving Electron an explicit name
worth reading in Task Manager's **Startup apps** list is the tidy way to make the
two distinct on purpose rather than by accident; whichever name is chosen, the
distinctness is the thing to test, because nothing else protects it.

**Nothing here fails.** A take-over that could not be made leaves the value where
it was and says so in the log, and the next launch tries again: a Verkstead that
would not serve because a value from the version before it could not be deleted
would be the worse of the two — exactly as the Mac arm has it.

**On `win32`, and only where there is a registration to make.** `startup.ts`
answers `nowhere` on an unpackaged run by decision, so a checkout run that deleted
a developer's Run value while offering them no box to tick would be the worst of
both.

**Reading and deleting a registry value is the one thing this app has no API for.**
Electron has none, and this app has no dependencies at all and is not the place to
acquire a native one — so it is a program run and its output read, injected the way
`LoginItem` is so that every arm is an ordinary vitest on the Linux runner rather
than a reach into the platform. Which program and how its absence or its silence is
read is this task's to settle; what matters is that a value that is not there, a
key that is not there and a command that would not run are the same answer, which
is the reading every other arm gives an unreadable registration.

**Then the hidden sign-in start, on a real Windows.** The registration is a command
line here, so `--hidden` reaches `process.argv` and `hidden()` is the whole of the
reading — no `wasOpenedAtLogin` and nothing deprecated, which is the Mac's problem
rather than this one. A sign-in start comes up with no window while there is an icon
in the tray to reach the app by, and with a window where there is not. Only a real
sign-out and back in exercises it.

## Acceptance criteria

- [ ] A profile carrying the tray app's `net.tobico.Verkstead` Run value comes up
      registered through the login-item API with that value gone and the box
      reading on; one carrying none is untouched; and an unpackaged run touches no
      value at all — every arm a vitest on the Linux runner.
- [ ] The value the API writes is not the one the take-over reads, and a second
      launch after a take-over leaves the registration standing rather than
      removing it.
- [ ] The box on the Desktop page writes and removes a value naming the app's own
      path on a real Windows, reads back what it wrote, and survives a launch of an
      app that has moved by naming where it is now.
- [ ] With the box ticked and the tray icon on, a real sign-out and back in brings
      Verkstead up with no window on the screen and its log says so; with the icon
      off, the window comes up.
