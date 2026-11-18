# The framework and the Plugins page

A directory with a `plugin.yaml` at its root is a **Plugin**, and Verkstead
loads one. The manifest's grammar is written down and read, and
`verkstead plugin check <dir>` says whether a directory is a plugin Verkstead
would load, and why not. Code runs as a wasm component under wasmtime and the
component model, reaching only what its manifest asks for. No Category is behind
any of it yet, so this stage proves the loading, the versioning and the trouble
reporting on fixtures, plus the release path that embeds the same component
bytes in every platform's binary.

The settings page gains a **Plugins** section at the foot of the list, beside
Agent Profiles and Repos. It has two groups, Installed above Built-in. Each row
has a checkbox, and a trouble line where a plugin failed to load or a dependency
is missing, disabled or outdated. **Add a plugin…** takes a git URL or a browsed
path and shows a consent card with what the plugin provides and the
capabilities it asks for. **Install** grants them. Git rows have **Update**.
**Remove** is refused while a dependent stands on the plugin. Opening a row draws
the plugin's declared settings fields. The installed list is a Replicated
record, and every device clones the source for itself. The decisions are
ADR-0030's and Question Sets 1760 to 1772.

Roadmap stage: [01: The framework and the Plugins page](docs/roadmaps/plugins/01-the-framework.md)

## Tasks

- [ ] 01: The manifest and `verkstead plugin check` — [details](01-the-manifest.md)
- [ ] 02: The toolchain moves to Rust 1.97, with wasm32-wasip2 — [details](02-the-toolchain.md)
- [ ] 03: Running a wasm component — [details](03-running-a-wasm-component.md)
- [ ] 04: Built-in components embedded in every binary — [details](04-built-ins-embedded.md)
- [ ] 05: Add a plugin from a local directory, and the Plugins section — [details](05-local-install-and-the-section.md)
- [ ] 06: Install from git, and Update — [details](06-git-install-and-update.md)
- [ ] 07: Dependencies — [details](07-dependencies.md)
- [ ] 08: Disable and Remove over dependents — [details](08-disable-and-remove.md)
- [ ] 09: The settings pane — [details](09-the-settings-pane.md)
