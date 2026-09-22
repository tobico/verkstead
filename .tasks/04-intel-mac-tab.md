# 04. An Intel Mac is told apart, and gets its own tab

## What to build

Homebrew has dropped Intel Macs: its installer aborts with *Homebrew on macOS
is only supported on Apple Silicon processors!*, and its formulae there get no
bottles. So on an Intel Mac the wizard's automated install fails at the
elevated step that makes the prefix (`chmod /usr/local` is refused — the
directory is a firmlink root on every Catalina-or-later Mac — and the
installer would have refused after it), and the hint screen tells the human to
run the same installer by hand. ADR-0016 (*Macs*) settles that **an Intel Mac
is not Homebrew's**: it is told apart, it has a tab of its own, and its planner
runs no `brew`.

**Telling it apart.** `Machine` learns the architecture the way it learned
`/etc/os-release`: read once on the real machine, stated in tests. On macOS
the fact is `hw.optional.arm64` through `sysctl`, which answers for the Mac
rather than for the slice the server happens to be running as — a universal
binary under Rosetta is still on an Apple-silicon Mac, and Homebrew's prefix
there is still `/opt/homebrew`. The distro of an Intel Mac is a new
`Distro::MacOsIntel` beside `MacOs`, carried through the render crate to the
viewer's generated types. Everything that switches on `Distro` has to learn
the second Mac: the planner's dispatch compares against `Distro::MacOs` by
equality and would otherwise hand an Intel Mac the Linux packager's *nothing*;
the tab order the viewer draws is a plain list a missing entry silently
shortens; and the guides record is what the type checker holds exhaustive. Add
a test that the viewer's tab list covers every `Distro`, since nothing does.

**The Intel tab.** Drawn ninth, detected on an Intel Mac, reachable from every
other. No Homebrew line above its rows, and one sentence saying why this tab
differs from the other Mac's — Homebrew no longer supports Intel Macs. Its
rows:

- Sandbox — Apple's `sandbox-exec`, as on the other Mac tab.
- git — `xcode-select --install`, Apple's own dialog.
- Claude — Anthropic's installer, the same row task 03 gave the other Mac.
- Codex — a link to its GitHub releases, with a note that the binary goes in
  `~/.local/bin`, which every Mac session looks in.
- Grok — as on every tab.
- OpenCode — its own installer, `curl -fsSL https://opencode.ai/install | bash`,
  which lands in `~/.opencode/bin`.
- gh — a link to its GitHub releases with the same `~/.local/bin` note; task
  05 makes the planner install it, and the row's link stays for the machine
  where that could not run.

**The Intel planner.** No `brew` line and nothing raised: Claude, Grok and
OpenCode are vendor units run as the user, each landing under the home and
written to `session_path` (OpenCode's is a new vendor beside Anthropic's and
xAI's, landing in `.opencode/bin`; the same script also runs on Linux, but
this task only adds it for the Intel Mac). git, Codex and, until task 05, gh
go to the hint screen with a sentence each, the way a row with no command
already does. The Apple-silicon planner is unchanged. Nothing here can make
the `chmod /usr/local` failure again: the prefix step only ever runs on
Apple silicon, where `/opt/homebrew` is made fresh.

Mirror the installing suite's Mac test for an Intel Mac, and the planner's
unit tests for each row. `docs/adoption.md`'s Mac install paragraph says what
an Intel Mac gets instead of Homebrew.

## Acceptance criteria

- [ ] On an Intel Mac the wizard opens the Intel tab; nine tabs are drawn;
      the tab has no Homebrew line above it and says why in one sentence; a
      web test asserts the tab list covers every `Distro`.
- [ ] An Intel press with Claude, OpenCode and gh ticked runs Anthropic's and
      OpenCode's installers as the user, raises no dialog, lands both under the
      home and on `session_path`, and sends gh to the hint screen with its
      link — in the installing suite with stubbed installers, and in the
      planner's unit tests.
- [ ] A stated Apple-silicon machine, and a stated one whose process reads as
      x86_64 under Rosetta, both read `Distro::MacOs`; only a Mac whose
      `hw.optional.arm64` is absent or zero reads `Distro::MacOsIntel`.
- [ ] `adoption.md` says what an Intel Mac installs with.
