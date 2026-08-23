# 01. A session asks with the server's own binary

## What to build

Every Sandbox runs the Verkstead executable that is serving it, as `verkstead`,
in preference to whatever the machine has installed.

Today a Sandbox is given the host's `PATH`, so a session runs the
system-installed binary. That binary and the server are separate builds and
they have already disagreed: the installed one validates Question Sets locally
and requires `accepted_by` on a `proposal` block, a field the running server
refuses as unknown. The two halves cannot put a Proposal through together, which
means no grilling can reach its closing move without a hand-built CLI. The Guide
a session reads comes from the same stale binary, so it documents a schema the
server will not take.

One binary carries both verbs — `verkstead serve` and `verkstead ask` — so the
server already has on disk exactly what a session needs. Give the Sandbox that
one. Where the running server's own image cannot be found, say so plainly rather
than silently falling back to the machine's: a session asking with a binary
nobody chose is the failure this removes.

Nothing about the sandbox boundary widens. The binary is read-only inside, like
the Skills.

## Acceptance criteria

- [ ] `verkstead --version` inside a Sandbox, and the Guide it prints, come from
      the server's own build rather than the machine's install.
- [ ] A Question Set carrying a `proposal` written to the server's current shape
      is accepted when sent from inside a Sandbox, with nothing built by hand.
- [ ] A server that cannot find its own executable says which session it could
      not equip, instead of handing over the machine's binary.
