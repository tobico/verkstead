# settings-ui-tidy

The settings page has grown a paragraph under every control and a pane for
every Repo. This tidies it into sections a phone can scan: every section is a
heading, a plain checkbox where there was a painted switch, the configuration
that depends on a checkbox indented under it and greyed while it is off, and no
explanation beyond a field's label. Computed warnings and error lines stay
everywhere, because they say what the machine is doing rather than what a
control is for.

Two settings leave with it. A Repo no longer overrides how a conflicted pull
request is resolved, so the one Conflict resolution select on the Git pane is
the whole answer. And a sandbox bind is a directory every sandbox gets and
nothing else, so the `name=path` grammar goes from the settings, the startup
flag and the NixOS module alike, and the Repo panes that held those binds fold
into one list of names with a Remove beside each. Every task updates the docs it
touches — the design doc's settings block, ADR 0015, `docs/development.md`,
`docs/adoption.md`, `CONTEXT.md` and the NixOS module's option text — in the
same change.

## Tasks

- [x] 01: The per-repo conflict override goes end to end — [details](01-per-repo-override-goes.md)
- [x] 02: A plain checkbox with a nested group, proven on Language support — [details](02-checkbox-and-language-support.md)
- [x] 03: The Git section — [details](03-the-git-section.md)
- [x] 04: Cleanup nests its days under two checkboxes — [details](04-cleanup.md)
- [x] 05: Remote access is a checkbox that will not lock you out — [details](05-remote-access.md)
- [x] 06: One Repos page — [details](06-one-repos-page.md)
- [x] 07: Repo-scoped binds go end to end — [details](07-repo-scoped-binds-go.md)
- [x] 08: Paths becomes Sandbox binds — [details](08-sandbox-binds.md)
- [x] 09: Agent profile cards — [details](09-agent-profile-cards.md)
