# 03. The words

## What to build

Make the repository read as if the tray app never shipped, except where history
says it did.

**The documentation that says the crate is still here.** The development guide
has a paragraph naming the default-on feature, the headless builds that turned
it off and the GTK the workspace linked, plus four console blocks in its
packaging section that still build the CLI with `--no-default-features`. Those
go. The release guide needs nothing: it already reads for the eight legs of the
shape that ships now.

**The server crate's own accounts of why it does what it does.** The module that
runs a program with no console window rests its whole reason on the tray app's
Windows shim having started the server with `CREATE_NO_WINDOW`. The behaviour is
unchanged and still wanted, so the reason has to be re-grounded on what starts
the sidecar today — which is a claim to check against how the Electron app
spawns it, not one to reword blind. The display, elevation, remote and sandbox
modules each name the tray app for something it used to do.

**And the Electron app's pointers into the crate that is gone** — about twenty
of them, across the app's sources and its vitest suites: the launch agent and
Run key take-overs, the log file's format, the menu bar artwork, the tray menu's
three items, the opener, the window and the startup registration each say which
Rust file they were written against. That prose is load-bearing, because the
take-over still reads a format whose only other statement was that Rust. So it
stays and **names the tray app in words, with no path**.

Then the records. The old desktop roadmap's index note describes what stands
today and has to say the app it shipped is gone; this roadmap's own note about
the trunk staying releasable is now describing something finished. ADR-0012
**gains a closing line saying what it described no longer exists** — its status
line already says it is superseded, and this is where the code behind it stops
being there. Finally sweep the glossary for anything that reads as current
rather than as a take-over the packed app still performs on an old machine.

## Acceptance criteria

- [ ] A grep for `verkstead desktop` finds only ADR-0012, the old desktop
      roadmap's briefs and this roadmap's own briefs
- [ ] No file outside `docs/` cites a `crates/desktop/…` path
- [ ] The development guide carries no tray paragraph and no
      `--no-default-features`, and the account of why the server has no console
      window names what starts the sidecar today
- [ ] ADR-0012 says the code is gone, and both roadmap index notes say what
      stands
