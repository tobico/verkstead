# 02. Watched paths and repo registration

## What to build

Verkstead learns which parts of the filesystem it is allowed to touch, and
gets its first record: the repos registered inside them.

**Watched paths** are configured in the environment at installation — a
server flag and environment variable, and an option on the NixOS module —
never discovered by scanning. They are a security boundary rather than a
convenience: a path outside them is refused, and the refusal is the server's
job, not the UI's, so no request can reach around it. Resolve symlinks and
`..` before deciding, so a path that merely reads as inside one is not
treated as inside it.

**Repo registration** adds a repo by absolute path. The server checks it is
inside a watched path, that it is a git repository, and records it with
whatever it needs to identify itself later — its path, a display name, and
its default branch. Registrations live in SQLite beside the question sets and
survive a restart.

The UI for this is a plain list with an add-by-path form. It does not need to
be the workbench yet; the next task builds the shell, and this list moves
into it. What it does need is to show a refusal as a refusal, with the reason
legible — a path outside the boundary, a directory that is not a repo, a repo
already registered.

The store has no migration machinery, and the design already settled on a
fresh database, so the new table goes into `apply_schema` alongside the
existing `CREATE TABLE IF NOT EXISTS` statements.

## Acceptance criteria

- [ ] Watched paths are configured by flag, environment variable and NixOS module option, and the server refuses to start with none set
- [ ] A repo inside a watched path registers, and appears in the list
- [ ] A repo outside the watched paths is refused, by the server, with the reason shown in the UI
- [ ] A path that escapes a watched path through a symlink or `..` is refused too, with a test covering each
- [ ] A directory that is not a git repository is refused
- [ ] Registered repos survive a server restart
- [ ] The phone's answering flow still works unchanged
