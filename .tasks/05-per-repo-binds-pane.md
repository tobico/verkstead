# 05. Per-repo binds on the Repo's pane

## What to build

Give each registered Repo's pane on the settings page its Sandbox
Configuration: the binds scoped to that Repo, editable where the settings own
them and read-only where the installation provided them.

The data is the `name=/abs/path` entries from tasks 01 and 03, keyed by the
Repo's registered name. The pane shows only its own Repo's; adding a row
there writes a `name=` entry for that Repo through the settings save, and
sessions of that Repo compose it in from the next spawn. Same row treatment
as the Paths pane: installer rows read-only and labelled, resolution
reported per row in words, the same brief word of caution beside the editor.

A Repo with no binds shows the empty editor rather than nothing — the pane
is where a human looks to learn the section exists.

## Acceptance criteria

- [ ] A bind added on a Repo's pane is mounted in that Repo's next session
      and in no other Repo's
- [ ] Installer-provided `name=` entries for that Repo draw read-only on its
      pane; settings-owned ones are added and removed there
- [ ] Resolution is reported per row, as the Paths pane does it
