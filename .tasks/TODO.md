# 01. Platform directories

`verkstead serve` started with no `--data-dir` puts everything it makes in the
platform data directory — `~/.local/share/verkstead` on Linux, `~/Library/Application
Support/Verkstead` on macOS, `%APPDATA%\Verkstead` on Windows — instead of
whatever directory it happened to be started from. One Data Directory still, as
it has always been, and one rule on every entry point: only the default moves,
and a developer running out of a checkout says `--data-dir .`. Nothing has been
released and the NixOS module already passes the flag, so nobody is broken and
no migration is written.

Beside it, a second resolution the stage does not itself use: the state/log
directory, where stage 02's desktop binary writes the server's log file,
because the stdout of a tray app launched from an icon goes nowhere. Both are
hand-rolled as functions of the environment values rather than of the process,
so all three platform arms are tested on the Linux runner — Windows is not
compiled in CI until stage 05.

Roadmap stage: [01: Platform directories](docs/roadmaps/desktop/01-platform-directories.md)

## Tasks

- [x] 01: The platform data directory, by default — [details](01-platform-data-dir.md)
- [ ] 02: The state/log directory, and the term for it — [details](02-state-log-dir.md)
