# 01. Skip a boundary step the account can already make

## What to build

When a session's sandbox boundary is built on Windows, the description grants
the session account a "step" — traverse plus read-attributes, non-inheriting —
on every ancestor directory on the way to each path the session may reach. This
exists so an agent can *resolve* a deep path (each prefix is asked for its
attributes) even though *reaching* it needs no such entry. But the step is
written on public directories the account can already resolve as an ordinary
member of Users — most consequentially the drive root `C:\`. Writing it there
via `SetNamedSecurityInfoW` makes Windows re-propagate inheritance across the
entire volume, which on an elevated host (the CI runner) costs minutes.

Make the step conditional: before writing it, determine whether the session
account can *already* traverse and read the attributes of that directory —
because its existing access-control list already grants those rights to the
account directly or to a group the account belongs to (Users, Authenticated
Users, Everyone). Where it can, write nothing. This narrows the boundary to the
directories that genuinely need an entry (the session's own private
directories and their non-public ancestors) and never touches `C:\` or other
public system directories on any machine.

The teardown path (revoking the boundary when a Conversation closes) benefits
for free: an entry that was never written is never revoked, so the matching
`SetNamedSecurityInfoW` on `C:\` at close — a second whole-volume walk — also
goes away.

This is the boundary logic of ADR-0014 (*Windows sessions*, amended: the
Sandbox is an account). Keep it a reviewable change of its own, and preserve
the existing behaviour that a step which cannot be written is not a session
refused — an ancestor the machine won't take an entry on is still an answer,
not a fault.

## Acceptance criteria

- [ ] Setting up a session boundary writes no access-control entry on `C:\` or on any public system directory the session account can already resolve
- [ ] A session boundary whose paths live under the drive root sets up in well under a second on Windows, where it previously took minutes
- [ ] A test asserts that no entry is written on the drive root (and on the public ancestors) for such a boundary — expressible on any platform the way the other `granting` tests read entries without writing them
- [ ] The Windows boundary and session suites (`sandbox_windows`, `sessions_windows`, `account_windows`, `launcher_windows`) still pass
- [ ] A step is still written where the account genuinely cannot already resolve an ancestor, and an ancestor that refuses an entry is still treated as an answer rather than a failed session
