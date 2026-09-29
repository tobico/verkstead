# Language descriptors

A language becomes a **descriptor** — data, in one grammar — and Rust is the
first one. The server loads its built-in descriptors from YAML embedded in the
binary, merges what `config.yaml` says over them key by key, and builds a
session's environment from the result. A Rust build inside a session behaves
exactly as it did before: the same variables, the same **Compile Server**, the
same directory. An installer who writes a descriptor for a language Verkstead
has never heard of sees its variables in the next session and its checkbox on
the settings page.

No new language ships in this stage. What ships is the system, and the proof
that it carries the one language already there. The decisions are
[ADR-0021](docs/adr/0021-language-descriptors.md)'s.

Roadmap stage: [01: Language descriptors](docs/roadmaps/language-caches/01-language-descriptors.md)

## Tasks

- [x] 01: Rust as an embedded descriptor — [details](01-rust-as-an-embedded-descriptor.md)
- [ ] 02: The languages map in `config.yaml` — [details](02-the-languages-map.md)
- [ ] 03: The settings page lists languages — [details](03-the-settings-page-lists-languages.md)
- [ ] 04: What will not load — [details](04-what-will-not-load.md)
- [ ] 05: The docs — [details](05-the-docs.md)
