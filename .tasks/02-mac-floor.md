# 02. A Mac session sees `~/.local/bin`

## What to build

A Mac app started from the Dock has launchd's `PATH` — `/usr/bin`, `/bin`,
`/usr/sbin`, `/sbin` — and a session's `PATH` is composed from the server's
own entries and then a fixed floor, so a `claude` that Anthropic's installer
put in `~/.local/bin` is found by neither the wizard's probe nor a session.
ADR-0016 deferred this on 2026-09-08 and settles it on 2026-09-22 (the *Macs*
section): **the Mac floor carries the home's `.local/bin`**, and on a Mac
**the floor's local installs lead the server's own entries**.

Two changes to how a Mac session's `PATH` is composed, both in the one place
the sandbox and the probes read it from:

- The Mac floor gains `~/.local/bin` at its head. The floor is a static string
  today and this entry is the server's home joined with `.local/bin`, so the
  composing has to own that entry rather than borrow it; a server with no home
  composes the floor without it. Linux's floor is unchanged.
- On a Mac the floor is two halves: the install half — `~/.local/bin`, the two
  Homebrew prefixes' `bin` and `sbin`, `/usr/local/bin` — is composed *ahead*
  of the server's own entries, and the system half (`/usr/bin` onward, nix's
  last) behind them, first occurrence still winning. From the Dock a session
  now resolves `/opt/homebrew/bin/git` ahead of Apple's `/usr/bin/git`, the
  way it does from a terminal whose `PATH` led with Homebrew. Linux keeps
  server entries first, floor last.

Nothing else needs a new rule: an entry on the composed `PATH` that lies under
the server's home is already put on the Mac policy read-only and executable,
and the wizard's program names already have their links followed into the
directory each lands in — `~/.local/bin/claude` into
`~/.local/share/claude/versions/` — on both Unixes. What changes is that the
entry is there to be granted. `session_path` still leads everything.

The tests that pin the floor need care rather than deletion: the one asserting
that a server started with no `PATH` composes the floor verbatim, the one
asserting every Apple floor entry survives composition, and the one asserting
every Mac floor entry lies under a directory the policy already allows — the
new entry is reachable through the per-user grant instead, and that is the
carve-out to state. Extend the Mac test in the per-user `PATH` suite to a
`claude` under `~/.local/bin` with *no* entry for it on the server's `PATH`,
and the real-Mac session test to assert `~/.local/bin` is on `$PATH` inside
`sandbox-exec` and ahead of `/usr/bin`.

The wizard's dependency step draws the session `PATH` it was composed with, so
the Mac tabs show the directory without further work; the restart note under
the list still holds for a `PATH` the human changes.

The Mac paragraph of `docs/adoption.md` (*sessions run on a Mac*) says what a
session reaches; say that `~/.local/bin` is among it.

## Acceptance criteria

- [ ] A Mac server started with launchd's `PATH` and a `claude` under
      `~/.local/bin` (a link into `~/.local/share/claude/versions/`) shows the
      wizard's Claude row present with that resolved path and its target, and
      a session runs it — asserted on a stated machine, and on a real Mac
      under `target_os = "macos"`.
- [ ] On a Mac the composed `PATH` reads `session_path`, then `~/.local/bin`
      and the Homebrew and `/usr/local` prefixes, then the server's own
      entries, then the system directories; on Linux it reads exactly as
      before, and the floor tests say so for both.
- [ ] A Mac server whose environment names no home composes the floor without
      the entry and starts.
- [ ] ADR-0016's *Macs* section is what the code does; `adoption.md`'s Mac
      paragraph names `~/.local/bin`.
