# Languages as plugins

The seven descriptors in `languages.yaml` become six built-in **Language**
plugins — Rust and C/C++ one, because they share the sccache Compile Server and
one mechanism is one thing to switch; Go, Node, Python, .NET and the JVM one
each — and the Plugins page has its first real rows. A Language plugin is pure
data: the descriptor grammar as it stands is the `language` block of a
`provides:` entry, and nothing a descriptor could not say before can be said
now.

The tick on the Plugins page becomes whether sessions get the language, and the
store's `size` a declared settings field drawn in the plugin's pane — both the
cluster's, taken up once from each device's `config.yaml` with on winning and
the larger size winning. The Sandbox page's language rows go, a language's
cache usage and Clear moving into its plugin's pane, and an installer's
`languages:` entry still merges over the plugin of the same name, or draws a
row of its own where no plugin provides that language.

Roadmap stage: [02: Languages as plugins](docs/roadmaps/plugins/02-languages.md)

## Tasks

- [x] 01: The `language` block and six built-in plugins — [details](01-the-language-block.md)
- [x] 02: The Built-in rows and the tick — [details](02-the-rows-and-the-tick.md)
- [x] 03: The size field — [details](03-the-size-field.md)
- [x] 04: The cache panel in the plugin's pane — [details](04-the-cache-panel.md)
- [ ] 05: The installer's languages — [details](05-the-installers-languages.md)
