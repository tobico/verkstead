# A plugin harness on Windows

The Agent interface stage made a harness a plugin can declare, and proved it
end to end on Linux only: every suite that launches a session under the stub
harness plugin is `#![cfg(unix)]`. The review of the stage found that a
Windows session's Transcript under a plugin harness is probably unreadable, and
the human chose to have that proved and fixed on Windows as work of its own.

## Tasks

- [x] 01: Prove and fix a plugin harness on Windows — [details](01-a-plugin-harness-on-windows.md)
