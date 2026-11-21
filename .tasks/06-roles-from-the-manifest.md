# 06. Roles from the manifest

## What to build

Roles are the Process's (Set 1765, Q26). The `roles` its manifest declares —
name, label, one the **building role** — decide which Pairings a Conversation
asks for, what the Agent control draws, and which session the Done signal,
Memos and previews treat as building.

- The store's three-variant `Role` becomes a word keyed by role name. The
  three names Verkstead knows — `grilling`, `implementation`, `review` — keep
  their per-Repo memory and platform defaults with nothing rewritten; a name
  nobody has used gets a memory of its own the first time it is picked.
- Each state's sessions run under the role its state names (a state option,
  or the Activity's default where the state says none), and steer targets
  offer the roles of their target state — replacing `steering.rs`'s
  `roles(target, process)` table.
- The Agent control's shape follows the role count: one role is a flat
  dropdown, more a panel, labelled with the manifest's labels — ADR-0020's
  table becomes the manifests'. Update ADR-0020 (or note in ADR-0030) that the
  table now lives in the manifests.
- The five built-ins declare the roles they use today, so nothing a human
  sees changes for them.

## Acceptance criteria

- [ ] A fixture Process with one role draws a flat dropdown, one with three a
      panel, each with the manifest's labels
- [ ] A role name never used before is remembered per Repo once picked; the
      three known names keep the memory already stored
- [ ] Memos and previews are accepted from the building role's session and
      refused from another's, on a fixture whose building role is not
      `implementation`
