# 03. A sandbox under the unit

## What to build

The packaged systemd unit can host a sandbox. Today it cannot: the service's
hardening is written for a server that only answers HTTP, and bwrap needs to
unshare namespaces the unit forbids outright.

Stage 01 already dealt with the half of this the stage brief remembered — the
watched paths are bound through `ProtectHome = "tmpfs"` and the VM test already
proves a Repo under `/home/watched` registers. What is left is narrower, and
what actually blocks bwrap has to be **established rather than assumed**: the
candidates are `RestrictNamespaces`, `PrivateUsers`,
`CapabilityBoundingSet`, `SystemCallFilter`'s `~@privileged`,
`MemoryDenyWriteExecute` and `ProtectProc`, but which of them matter, and how
far each has to open, is something the VM test is the instrument for. Relax
what the evidence says needs relaxing and no more, and comment each relaxation
with what needs it — a hardening line removed without a reason recorded is one
nobody can ever put back.

A unit that cannot spawn a sandbox is a unit testing the wrong product, which
is why this is done now rather than when the first real session runs. Extend
`nix/vm-test.nix` with a subtest that starts a sandbox under the running
service and has its probe report back.

The worktrees of task 01 live under the state directory, so `ProtectSystem =
"strict"` already permits them — worth confirming under the unit rather than
inferring, since that is the difference between the crate tests and this one.

## Acceptance criteria

- [ ] The VM test starts a sandboxed probe under `verkstead.service` and the
      probe reports the surface task 02 defined
- [ ] Every hardening setting that was relaxed carries a comment naming what
      needs it, and settings that turned out not to block bwrap are left alone
- [ ] The VM test creates a worktree under the state directory from inside the
      running service
- [ ] The rest of the VM test still passes unchanged
- [ ] `nix flake check` passes
