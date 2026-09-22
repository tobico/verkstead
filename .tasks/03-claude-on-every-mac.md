# 03. Claude is Anthropic's installer on every Mac

## What to build

The Mac planner installs Claude Code as Homebrew's `claude-code` cask, and the
Mac tab of the hint screen shows that line, both for one reason: the Dock's
`PATH` never named `~/.local/bin`. Task 02 put that directory on the Mac
floor, so the reason is gone, and ADR-0016 (*Macs*) settles that **Claude is
Anthropic's installer on every Mac** — the install that stays current, and
the one every other tab leads with. The cask goes entirely: not as the
planner's line and not as the tab's alternative.

- The Mac planner's Claude row becomes the vendor unit the Linux planner
  already runs — Anthropic's `curl … install.sh | bash` as the user, landing
  in the home's `.local/bin` and writing that directory to `session_path` the
  way it does on Linux (the floor has it too; writing it is what the row's
  *lands* has always meant, and it costs nothing). A press with Claude alone
  ticked on a Mac with no `brew` wants no Homebrew: the prefix and installer
  units are raised only in front of a `brew` line, and there is none.
- The Mac tab's Claude row is Anthropic's line with a note of the Mac's own:
  no caveat about the shell's `PATH` or a restart, because `~/.local/bin` is
  on every Mac session's floor. The Homebrew line drawn above the Mac tab's
  rows stops claiming that *every* command below is Homebrew's — it is what
  the rows that are Homebrew's want first.
- The end-to-end installing suite's Mac test, which ticks git and Claude and
  asserts two `brew` installs, becomes one `brew` install and one vendor
  installer, with `curl` stubbed the way the vendor-installers suite already
  stubs it; the planner's unit tests follow.
- `docs/adoption.md`'s Mac install paragraph (*On a Mac it is Homebrew's
  instead*) says Claude is the exception and why.

## Acceptance criteria

- [ ] Ticking Claude on a Mac runs Anthropic's installer as the user, raises
      no dialog, and the row is present at the next probe with `~/.local/bin`
      resolved — in the installing suite with a stubbed `curl`, and in the
      planner's unit tests for a Mac with and without `brew`.
- [ ] The word `claude-code` appears nowhere in the planner, the tab, or the
      web tests; the Mac tab's Claude row shows Anthropic's line with a note
      that says nothing about the shell's `PATH`.
- [ ] The Homebrew line above the Mac tab's rows is worded for the rows that
      are Homebrew's, and `adoption.md`'s Mac paragraph names the exception.
