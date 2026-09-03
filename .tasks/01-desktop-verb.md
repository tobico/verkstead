# 01. The desktop verb

## What to build

`verkstead-cli` gains a default-on `desktop` cargo feature carrying the desktop
library as an optional dependency, and a `desktop` verb that is the tray app
end to end: settle the address first, then run — `--no-open`, the server's own
flags, the refusal dialogs, the rotating log file, all exactly as
`verkstead-desktop` does them today. Bare `verkstead` still prints the Guide,
and nothing about the other verbs changes whether the feature is on or off.

Launch on Startup must register the invocation that is actually running: the
desktop library is told at entry what to re-register with rather than reading
it off the executable alone, so an app entered through the verb writes an entry
carrying `desktop --no-open`, while the old binary — still alive until task
02 — goes on writing what it writes today.

`scripts/dev.sh` starts the app through the verb.

## Acceptance criteria

- [ ] `verkstead desktop --data-dir …` behaves as `verkstead-desktop` does
      today: server up, tray drawn, viewer opened unless `--no-open` says
      otherwise, failures said on stderr, in the log file and in a dialog.
- [ ] The same build answers `guide`, `ask`, `answers` and `serve`, and bare
      invocation prints the Guide.
- [ ] `cargo build -p verkstead-cli --no-default-features` compiles and links
      no GTK.
- [ ] Launch on Startup chosen in a verb-started app writes an entry that
      names the running executable plus the verb.
