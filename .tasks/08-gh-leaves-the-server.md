# 08. `gh` leaves the server

## What to build

The server stops running `gh` anywhere. `gh` is still lent into the sandbox as
the Forge's declared tool.

- **The suite's stand-in becomes HTTP.** Every server test that drives a fake
  `gh` script drives task 07's HTTP stand-in through the outbound redirect
  instead. That is about a dozen files in the suite (sessions, conversations,
  repos, settings, sharing, onboarding, accounts, ports, transfer, ui_content,
  logins) and the unit tests in checks, comments, stacks, publishing and
  wrapping. The router constructors that take a `Gh` take the stand-in instead.
- **The `gh` implementation goes**: `Gh`, `Trouble::read` and the Mac-only
  lookup of `gh` on `sandbox::machine_path`. Git's credential, now built from
  the Forge's declared username and the Author's token for that Forge, moves
  out of `github.rs`.
- **The `gh stack` check before a stack's fix session** (Set 1878, Q4) reads
  the extension's directory under the session's home (`gh/extensions/gh-stack`
  in gh's data directory) instead of running `gh stack --help`. A missing one
  still stops the run with the same Notice before a go is spent.
- Make sure no leftover path execs `gh`: the onboarding prefill (task 03), the
  extension probe and the host-login fallback.

## Acceptance criteria

- [ ] Every test that drove the fake `gh` passes against the HTTP stand-in,
      and no test constructs a `gh` program.
- [ ] The server binary never execs `gh`. No `Command` in the server names it,
      outside the sandbox's lending of the tool.
- [ ] A stack whose session home lacks `gh-stack` stops with the Notice naming
      it and its install line, with no session sent.
