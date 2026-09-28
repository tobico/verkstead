# 03. Repo matching across devices

## What to build

A reading that answers, of a Repo here and a device over there, **which of that
device's Repos is this repository** — the question a cluster has to settle
before anything keyed by a path can cross between machines.

**Origin URL first.** The `origin` remote's URL as git gives it, compared after
the differences that are spellings rather than repositories are taken out: a
trailing `.git`, a trailing slash, and the `scp`-style spelling of an SSH URL
against its `ssh://` form. Nothing about the registration is trusted for this —
a Repo records a path, a name and a default branch and no origin at all, so both
ends ask git afresh, which is how everything else here reads a repository.

**Then by name, where neither has an origin.** The registered Repo's name, which
is the directory's own. A Repo with an origin never matches one without: they
are two repositories until something says otherwise, and a shared name is not
that something.

**Asked over the link from either end.** B asks which of A's Repos is this one —
which is what the memory sync in task 06 needs, to know whose entry to pull —
and the same question is asked the other way round by transfer later. A device
that is not answering is refused by name rather than answered *no match*: those
are two different things to say, and a preflight that confused them would send
somebody to Open repo on a machine that already has it.

**No match is an answer.** It is not a failure here: the caller decides what to
do without one, and this stage's caller carries on with nothing pulled.

## Acceptance criteria

- [ ] One origin checked out at two different paths under two different names
      matches across the devices.
- [ ] Two Repos sharing a name with different origins do not match, and two
      spellings of one URL do.
- [ ] Neither end has an origin and the names agree: a match. One has an origin
      and the other has not: no match.
- [ ] Asked of an unreachable device, it is refused naming the device rather
      than answering that nothing matched.
