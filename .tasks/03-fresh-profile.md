# 03. The fresh profile

## What to build

A Windows session starts in a profile of its own rather than in the human's.
`Homes` gains a Windows answer: a directory per Conversation under the Data
Directory, made fresh as each session starts, the way it already does on a Mac —
and `USERPROFILE`, `HOME`, `APPDATA`, `LOCALAPPDATA` and `TEMP` all point into
it, so npm's caches, tool state and temporary files stay out of the real one.

Built now rather than with the container, because it is what the container's
grants will be made against, and because a session that wrote into the real
profile unsandboxed would be leaving state behind that no later stage could take
back.

**The Profile's account is joined in, and the rule is over the account rather
than over one backend.** Four agent types keep an account four ways — Claude's
pair at `.claude` and `.claude.json`, Codex's `.codex`, Grok's `.grok`,
opencode's config and data directories — and the Surface already names whichever
of them the Profile holds. So the rule is written over what the description
says, not over Claude's pair: **every directory in the account is joined in by a
directory junction**, which needs no privilege, and **every file by a hard
link**. A file symlink on Windows needs a privilege a per-user install has not
got, which is why the link is a hard one.

The open rendering is where this happens, the way the seatbelt renderer makes
its fresh home and its links as it renders: what the description calls an empty
directory is really made and emptied, and what it calls a path found somewhere
else is really joined in.

**A hard link needs one volume**, and the end that can differ is the account's:
the fresh profile is made under the Data Directory and so is never elsewhere,
where an account is wherever the Agent Profile points inside a Watched Path,
which may well be another drive. A machine whose Data Directory and whose
account's own directory are on different volumes **refuses the session**, with a
line saying which two paths and why — the way a Sandbox that cannot be built
refuses one.

Directories are not in that rule: a junction crosses volumes and needs nothing.

## Acceptance criteria

- [ ] Inside a session the Profile's account is the real one — a file the
      account holds is readable inside and a file written inside is on the
      account — asked of a Claude Profile *and* of a type whose account is one
      directory, so the rule is proved over the account rather than over
      Claude's pair.
- [ ] A file written to `%TEMP%` inside lands under the fresh profile, and
      `USERPROFILE`, `HOME`, `APPDATA` and `LOCALAPPDATA` all resolve inside it
      rather than into the human's own.
- [ ] The profile is fresh: what one session left in it is not what the next
      session of the same Conversation finds.
- [ ] An account on a different volume from the Data Directory refuses the
      session, naming both paths and why, rather than starting one whose account
      is missing.
