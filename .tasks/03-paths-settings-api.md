# 03. The settings API tells and takes Paths

## What to build

Carry watched paths and sandbox binds through the settings API: the one
GET/POST at `/api/ui/settings` that already moves the git author, the token,
the build cache and the share viewer URL.

The view side reports, per entry, everything the page will need to draw it:

- **Its source** — the installation's (CLI flag or env var) or the
  settings' own. Installer entries are read-only on every surface; only
  settings-owned entries are editable, and a save writes only those.
- **Whether the server can currently see it** — the per-entry resolution
  report settled in the grilling: for a watched path, that it resolves to a
  directory; for a bind, that the path exists. This is also how a nix user
  learns an entry needs an installer change before it can function, so the
  distinction worth reporting is *resolves* against *does not*, with the
  reason in words.
- For binds, **which Repo it is scoped to**, where it is a `name=` entry.

The edit side takes the settings-owned lists as values, the way the share
viewer URL travels: what is sent is what `config.yaml` holds afterwards, and
a save always lands whatever the entries resolve to — resolution is reported
back with the saved view, never a refusal. Keep the shape in the CLI grammar
(strings), matching what task 01 and 02 store.

Wire types live where the existing settings types do and export to
TypeScript the same way; regenerate the generated types. Server needs access
to the CLI-provided sets from the settings handler to label sources — thread
them through app state alongside what is already there.

## Acceptance criteria

- [ ] GET returns every watched path and bind — installer and settings alike —
      each labelled with its source and whether the server can see it now
- [ ] POST replaces the settings-owned entries in `config.yaml`, leaves
      installer entries untouched, and lands even when entries do not resolve
- [ ] The generated TypeScript types carry the new shapes, and server tests
      cover source labelling and resolution reporting
