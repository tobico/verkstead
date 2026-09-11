# 01. A directory Verkstead installs into stays on the session PATH

## What to build

A new `config.yaml` key, `session_path`: a list of directories that a session's
`PATH` is composed with **ahead of the server's own entries**, so a Claude Code
that Verkstead installed into `~/.local/bin` shadows a stale distribution one in
`/usr/bin`. The list is read at startup beside the server's `PATH`, and the
same three rules apply to it — first occurrence wins, only rooted entries, and
an entry neither under the server's home nor under the platform floor is
dropped and logged. On Windows it is put ahead of the server's `PATH` as it
stands, that platform having no floor.

**The held startup value can grow inside a run.** Today the composed `PATH` is
read once and held where the sandbox builder and the wizard's probes both read
it. Keep that one value, and give it one way to change: an install that lands
in a directory appends that directory, writes it to `session_path`, and the
next probe and the next session both see it — no restart. Nothing else moves
the value; the server's own `PATH` is still what it was started with.

The wizard already draws where a session looks from the wire, so a configured
entry appears in that list with nothing else to do. The settings page does not
grow a field for it in this task: the key is written by the installer in task
04 and editable by hand, which is the *told, not found* rule the settings module
already keeps.

ADR-0016 carries the amendment, made in the plan commit.

## Acceptance criteria

- [ ] With `session_path: [~/.local/bin]` configured and a stub `claude` there,
      the wizard's Claude row reads Present at that path, a session's `PATH`
      names the directory ahead of the system ones, and it is bound read-only.
- [ ] An entry appended mid-run reaches the next probe and the next session
      without a restart, and is in `config.yaml` at the next start.
- [ ] An entry outside the home, such as `/opt/foo/bin`, is dropped and logged,
      and a malformed key reads as no entries, the way every other key does.
