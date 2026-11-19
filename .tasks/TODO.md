# Codex across

Codex becomes the first built-in harness to be what a user writes: a directory
under `plugins/` with a `plugin.yaml` and a wasm component, embedded in the
binary and listed under **Built-in** on the Plugins page, with the in-process
Rust implementation of the `Harness` trait deleted. A Codex Profile is made,
paired and launched as before; it is nudged over store-and-nudge; its rollout is
drawn on the Timeline; a Conversation resumes it by the id it chose for itself;
declared MCP servers reach it through its `config.toml`; its memory syncs to a
device it is not bound on.

Re-grounding found that stage 03's plugin harness could not yet be Codex — its
`parse-transcript` needs a session id Codex never takes, a plugin harness syncs
no memory, the grammar refuses Codex's resume shape, and a built-in plugin never
stands up a harness — so the interface grows first (01, 02, on the stub
harness), and Codex crosses after (03–05). Breakdown approved in Question Set
1836.

Roadmap stage: [04: Codex across](docs/roadmaps/plugins/04-codex-across.md)

## Tasks

- [x] 01: A plugin harness that names itself — [details](01-a-plugin-harness-that-names-itself.md)
- [x] 02: The rest of the grammar Codex needs — [details](02-the-rest-of-the-grammar-codex-needs.md)
- [x] 03: Codex's plugin, manifest and component — [details](03-codex-plugin-manifest-and-component.md)
- [ ] 04: The crossing — [details](04-the-crossing.md)
- [ ] 05: Codex's install and onboarding row from its manifest — [details](05-codex-install-and-onboarding-from-its-manifest.md)
