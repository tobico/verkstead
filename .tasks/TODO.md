# macOS release fixes

The v0.1.2 dmg has four problems on a Mac, three of them one problem: the app
in Applications draws the generic icon; the wizard does not see a `claude` or
`gh` in `~/.local/bin`; the automated install fails at `chmod /usr/local`; and
the manual tab tells an Intel Mac to use Homebrew, which has dropped Intel —
its installer refuses the machine outright. ADR-0016's *Macs* section (amended
2026-09-22) records what was settled: `~/.local/bin` on the Mac floor with the
local installs ahead of launchd's directories, Anthropic's installer for
Claude on every Mac, an Intel tab and planner with no Homebrew in it, gh from
GitHub's release zip, and a git row that checks for Apple's command line
tools.

Six sequential slices, smallest first. Each is demonstrable on a Mac by
itself and carries its own tests; every one is unit-tested on a stated
`Machine` so it runs on any runner, with the real-Mac assertions gated on
`target_os = "macos"` the way the existing suites are.

## Tasks

- [x] 01: The bundle draws its icon — [details](01-bundle-icon.md)
- [x] 02: A Mac session sees `~/.local/bin` — [details](02-mac-floor.md)
- [x] 03: Claude is Anthropic's installer on every Mac — [details](03-claude-on-every-mac.md)
- [x] 04: An Intel Mac is told apart, and gets its own tab — [details](04-intel-mac-tab.md)
- [x] 05: gh on an Intel Mac comes from GitHub's release — [details](05-gh-from-the-release.md)
- [ ] 06: git on a Mac counts only with the command line tools — [details](06-git-needs-the-tools.md)
