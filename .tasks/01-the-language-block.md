# 01. The `language` block and six built-in plugins

## What to build

`crates/server/languages.yaml` is retired as a file, and its seven descriptors
become six built-in plugins under the repository's `plugins/`, embedded the way
the fixture already is: **Rust and C/C++** in one plugin (both name the
`sccache` capability, and nothing else does), and **Go, Node, Python, .NET and
the JVM** one each. Each plugin is manifest-only — no component, no WASI
`capabilities:`, since a Language runs nothing — with fixed uuids, and one
`provides:` entry of category `language` per language it provides.

**The block is the descriptor grammar unchanged.** Every key a descriptor has
today (`label`, `detect`, `env`, `capabilities` naming server behaviour such as
`sccache`, `stores`) keeps its name and meaning inside the entry, plus the
language's name, which is what an installer's `languages:` entry keys by. So
no installer's `config.yaml` needs a key renamed. A sketch of the shape the
task settles:

```yaml
provides:
  - category: language
    language: rust          # the name `languages:` in config.yaml keys by
    label: Rust
    detect: [Cargo.toml]
    env: { CARGO_HOME: "{cache}/cargo" }
    capabilities:
      sccache:
        env: { RUSTC_WRAPPER: "{sccache}", ... }
    stores: { ... }
```

The manifest reader stops leaving a `language` entry's body unread: it is read
in the descriptor grammar and refused by field the way the rest of
`plugin.yaml` is, including a key nothing knows and a variable the Sandbox sets
itself. `docs/plugins.md` gains the Language grammar, carrying over what
`languages.yaml`'s header comments document today (placeholders, stores, units,
capabilities). What every session is built from is now the built-in plugins'
blocks, with `config.yaml`'s `languages:` merged over them key by key exactly as
before (`null` takes a variable out), and `enabled`/`size` still read from
`config.yaml` for now — later tasks move them.

The module docs, `CONTEXT.md`'s **Descriptor** entry and ADR-0021's pointers
that name `languages.yaml` are brought up to date.

## Acceptance criteria

- [ ] A session under each of the seven languages gets byte-for-byte the variables, stores and sandbox grants it got before (the existing session and package-store suites pass unchanged in what they assert).
- [ ] All six embedded plugins pass `verkstead plugin check`; the Rust & C/C++ plugin names `sccache` once per language and no other plugin names it; a `language` entry with an unknown key or a Sandbox-owned variable is refused naming the field.
- [ ] An installer's `languages:` override of a built-in variable still wins, and one set to `null` still takes the variable out.
