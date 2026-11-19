# 01. Prove and fix a plugin harness on Windows

## What is wrong, as far as anyone knows

A harness a plugin declares (`crates/server/src/harness/plugged.rs`, the sixth
`Harness`) has only been run on Linux. The suites that launch a session under
the stub harness plugin — `crates/server/tests/suite/sessions.rs`
(`Pickers::EverythingOnAPlugin`, `#![cfg(unix)]`), `stub_harness.rs`,
`logins.rs`'s plugin login and `installing.rs`'s plugin rows — have never run
on Windows, and nothing Windows-only was written for a plugin harness.

The one defect suspected, and not yet seen: **`parse-transcript` probably
cannot read the account on Windows.** The export is handed the account's root
as a string and finds the session's record under it (`plugin.wit`,
`parse-transcript: func(account: string, session: string)`). The host makes
that root readable by starting the component again with the directory
preopened under its own host path as the guest's name for it —
`Again::reading` in `crates/server/src/plugins/component.rs`:

```rust
grant.wasi.preopened_dir(dir, dir.to_string_lossy(), FsPerms::ReadOnly)
```

and `plugged::Reader` hands the export the same host path string as
`account`. On Linux that is `/home/…/.stub`, which wasi-libc resolves against
the preopen. On Windows it is `C:\Users\…\.stub`: a guest joining
`{account}/memory/{session}.jsonl` asks for `C:\Users\…\.stub/memory/…`, and
wasi-libc's preopen matching is by `/`-separated prefix, so it likely finds no
preopen and the read fails. The host then writes one `Entry::Unreadable` and
the Transcript reads as unreadable (`plugged::Entries`). The same may apply to
`write-configuration`'s and `login`'s instances if either is ever handed a
host path.

Likely fix, to be confirmed on the machine: preopen the account under a fixed
guest name (for example `/account`) on every platform and hand the export that
name rather than the host path. Update the WIT doc on `parse-transcript` and
`docs/plugins.md`'s description of what `account` is, and the stub component
(`plugins/tests/stub-harness/src/lib.rs`) if it assumes anything about the
string.

Everything else about a plugin harness on Windows is unproven rather than
suspected: the launch line, the root built of the account's paths (joined by
hard links and junctions on Windows — see `across_volumes` in
`crates/server/src/sandbox.rs`), the idle and signed-out phrases, the login on
a pseudoconsole, and the install commands, which run as one raised PowerShell
script of a block per command (`stepped` in
`crates/server/src/onboarding/install.rs`, never run on a real PowerShell 5.1).

## How to work it

This needs a Windows machine. The cluster's **winvm** is one this work may
move to: `verkstead transfer winvm`. The memories and `docs/development.md`
cover running the suite there; the server's Windows suites are
`sandbox_windows.rs` and `sessions_windows.rs`.

## Acceptance criteria

- [ ] A session under the stub harness plugin launches on Windows, and its
      Transcript is drawn out of what `parse-transcript` answered, live and
      after it ends — proved by a Windows suite test, not by reasoning.
- [ ] `parse-transcript` is handed an account name that resolves inside the
      component on every platform, and the WIT doc and `docs/plugins.md` say
      what that name is.
- [ ] A plugin's install commands, two or more of them, run under Windows
      PowerShell 5.1 as one raised script and stop at the first that fails —
      proved on the machine.
- [ ] The Linux suites still pass.
