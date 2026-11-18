# 01. The manifest and `verkstead plugin check`

## What to build

The `plugin.yaml` grammar in the server, a reader that refuses by field, and
`verkstead plugin check <dir>` over it. The check prints either that the
directory is a plugin Verkstead would load, or the refusal naming the field, and
exits non-zero on a refusal. ADR-0030's *The shape of a plugin* and CONTEXT.md's
**Plugin** and **Category** entries are the vocabulary. Read `docs/copy.md`
before writing the refusals: the same words reach the Plugins page later.

The manifest carries:

- **Metadata:** `uuid` (the identity), a display `name`, `author` (name and
  email), a semantic `version` (the workspace's `semver` crate), and an optional
  `repository` URL.
- **`interface`:** a whole number (Set 1772, Q6) naming the interface version the
  plugin was written against. This build knows a newest number, `1` for now. A
  higher number is refused, saying the plugin is for a newer Verkstead.
- **`description`**, and **`mark`**: an SVG, either inline or a path inside the
  directory. Checking that it is SVG is enough. Sanitising it is the viewer's job
  when it draws it.
- **`dependencies:`**: each entry has uuid, display name, author name and email,
  optional repository URL, and a **version specifier** (`semver::VersionReq`,
  such as `^2`). It is only parsed here. Task 07 resolves it.
- **`provides:`**: a list, each entry of one Category out of the six words
  (agent, process, forge, language, activity, skills). The body of an entry is
  not validated beyond its Category in this stage. Each later stage writes its
  own Category's grammar. An empty list is allowed: a manifest-only plugin that
  provides nothing is this stage's fixture.
- **`capabilities:`**: filesystem paths (with read-only or read-write), outbound
  hosts, and environment variables. A capability of an unknown kind is refused.
- **`settings:`**: fields of kind text, size, list or secret, each with a key
  and a label. An unknown kind is refused.
- **`component:`**: an optional path to a `.wasm` inside the directory. Task 03
  runs it. Here it only has to exist.
- **Per-platform overrides** keyed `linux`, `macos` and `windows`, merged over
  the common declaration for the platform being read for. Settle the merge rule
  (a key present in the override replaces the common one), write it down in the
  grammar's doc comment, and test it.

Unknown top-level keys are refused, naming them. A misspelt key silently
ignored is the gap the interface version exists to stop.

Write the grammar down where a plugin author will find it, as a doc beside the
other docs (for example `docs/plugins.md`), so later stages extend one place.

## Acceptance criteria

- [ ] A minimal manifest (uuid, name, author, version, interface, empty
      `provides:`) passes `verkstead plugin check`, which exits 0.
- [ ] Each of these is refused, naming the field: a missing uuid, a version that
      is not semantic, a dependency with no version specifier, an unknown
      Category, a capability of an unknown kind, a settings field of an unknown
      kind, an interface version ahead of this build, and an unknown top-level key.
- [ ] The check prints the refusal and exits non-zero. A directory with no
      `plugin.yaml` is refused as not a plugin.
- [ ] A `windows:` override replaces the common value when read for Windows and
      not when read for Linux.
