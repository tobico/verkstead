# 08. Disable and Remove over dependents

## What to build

**Remove** on an installed row takes the record off (a deletion stamp). Each
device then forgets the plugin, deleting its own checkout under
`plugins/<uuid>/`. A local plugin's own directory is never deleted. A built-in
row has **no Remove drawn**.

**Remove is refused while anything open stands on the plugin**, the refusal
counting them and leading there (ADR-0030). Write the "what stands on a plugin"
check so later stages add to it (Profiles on an Agent, Conversations on a
Process, Repos on a Forge). In this stage, installed plugins that **depend** on
it are the only things that stand, and the refusal names them.

**Disabling is allowed over standing things.** Unticking a plugin that others
depend on first says which ones go with it, and goes ahead on confirmation. Its
dependents fall into task 07's disabled-dependency trouble. Read `docs/copy.md`
for how the warning and the refusal are worded.

## Acceptance criteria

- [ ] Remove on a plugin nothing depends on removes it from every device, and
      its git checkout is deleted.
- [ ] Remove is not drawn on a built-in row (tested against a built-in fixture
      in the test build).
- [ ] Remove on a plugin another installed plugin depends on is refused, naming
      and counting the dependents.
- [ ] Unticking a dependency names its dependents before it is done.
