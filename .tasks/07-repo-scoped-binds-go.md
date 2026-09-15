# 07. Repo-scoped binds go end to end

## What to build

A sandbox bind is an absolute path every sandbox gets, and nothing else. The
`name=path` grammar, which gave a directory only to sessions working in the
Repo registered under that name, goes from every place it was read: the
settings entries, the `--sandbox-bind` startup flag, and the NixOS module's
sandboxBinds option.

The server stops composing a Repo's own set over the global one, and a
companion's own set with it: a sandbox for a Conversation binds the global set
and the companion checkouts, as before, and nothing per Repo. The parsed
configuration keeps no per-repo half. In the settings, a `name=path` entry is
dropped silently on read: it reaches no session and draws no row, so the
human's way to notice one is the config file itself. On the flag, a
`name=path` value refuses startup by name, the way a relative path or a missing
directory refuses today, since startup is the moment a service unit hears about
a bad entry. The NixOS module's bind-path helper no longer splits on `=`.

The paths view loses the Repo on an entry. The Paths page stops drawing strays
and the written-for label, and the unseen count on its card covers the global
entries alone, with the sentence about opening a Repo's pane gone. The
round-trip that wrote entries back keeps only the plain paths.

Docs follow: `docs/development.md` and `docs/adoption.md` describe both
grammars, `CONTEXT.md`'s Sandbox Configuration entry does, the design doc's
settings block explains strays and a Repo's Sandbox configuration heading, and
the NixOS module's option description says what `name=path` does. Each says
there is one grammar now.

Tests: the sandbox tests for per-repo and companion composition go or turn into
their global-only counterparts, the flag test refuses `name=path`, the paths
web tests drop strays and the written-for rows and cover a `name=path` entry
drawing nothing, and the settings tests' fixtures lose their Repo bind.

## Acceptance criteria

- [ ] A `name=/path` line in config.yaml reaches no session and draws no row on the Paths page, and the server starts without complaint.
- [ ] `--sandbox-bind name=/path` refuses startup naming the entry.
- [ ] A companion sandbox binds the global set and its checkouts only, and the Paths card's unseen count covers global entries alone.
