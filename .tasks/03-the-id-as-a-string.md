# 03. The id as a string, the account as paths

## What to build

`AgentType` stops being a closed enum in the store, the render crate and the
viewer. A harness is named by a string id, and a Profile's account becomes a set
of **labelled paths**.

- **Ids.** The five built-ins get fixed uuids, the way the fixture plugin has
  one. **A Profile record of a built-in keeps writing its old word** (`claude`,
  `codex`, `grok`, `opencode`, `agy`), so that a cluster device still on an
  older release reads it. A plugin harness's Profile writes the plugin's uuid.
  Either is read (Set 1786, Q2). The word or uuid is what `VERKSTEAD_AGENT`
  carries into a sandbox. An unknown id is a Profile whose harness is not here:
  it is broken (the rule CONTEXT.md gives under **Pairing**), not an error that
  stops a read.
- **Accounts as paths.** The replicated **binding** record (`profile_bindings`,
  per device) gains a map of path key → resolved path. Claude's pair is two
  declared paths, and the other four are one declared home each. **For a
  built-in, the old `claude_dir` / `config_file` / `home` fields are still
  written beside the map**, so an older device still reads the binding. A
  plugin harness writes the map alone (Set 1786, Q3). Old bindings and the
  provisional mirrors in the `profiles` table read into the map. There are no
  `claude_dir` / `config_file` columns on a live table to migrate: Profiles are
  Replicated records now, and that table only holds the pre-uuid mirrors.
- **The harness list.** The server serves the list of harnesses it can run,
  answered by the trait for the five built-ins. Each entry gives the id, the
  display name, the mark as SVG (the lobehub files in `web/src/marks/` move
  server-side), the declared paths with their labels and what each is for, and
  whether it logs in. Model lists stay where they are until task 08.
- **The viewer.** `ProfileAccount` stops being a tagged enum on the wire, and
  ts-rs regenerates `web/src/api/types.ts`. The Profile form draws one field per
  declared path, labelled as the list labels it. These hand-kept tables go:
  `HarnessMark`'s `MARK`, `AGENT_NAME`, `ACCOUNT_FIELDS`, `BLANK_ACCOUNT`,
  `AGENT_TYPES`, `loggable()`, `LoggingIn`'s `CALLED`, and `setup/Accounts.tsx`'s
  `LOGIN`, `KEPT` and `runnable()`. The mark is drawn from the list's SVG, made
  safe to draw. Onboarding's `Dependency`-keyed tables are task 09's.
  `KNOWN_MODELS` is task 08's.

Copy and words follow docs/copy.md and CONTEXT.md. Update CONTEXT.md's
**Agent Profile** entry where it says the account's shape is its type's.

## Acceptance criteria

- [ ] An existing Claude Profile, together with its binding written by the
      previous release, reads back with both paths. A Profile saved now still
      writes the old word and the old binding fields beside the map.
- [ ] No `enum AgentType` remains in store, render or viewer, and the listed
      viewer tables are gone. The form, the cards and the pickers draw names,
      marks and path fields from the server's harness list, and look as they did.
- [ ] A Profile naming an id this server does not know is drawn as broken, and
      is not offered on a picker.
