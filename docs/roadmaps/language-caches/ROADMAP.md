# Language caches roadmap

The **Build Cache** is Rust's alone today: one directory every Sandbox gets
writable, `CARGO_HOME` inside it, and the **Compile Server** for the compiled
half. A session building anything else starts cold every time — its `HOME` is
fresh and empty, so every package store a tool keeps under it is too. This
roadmap gives Node, Python, the JVM, .NET, Go and C++ what Rust has, and makes
a language a **descriptor** — data in `config.yaml`'s own grammar — so that
somebody installing Verkstead can add one without building it.

The decisions are in [ADR-0021](../../adr/0021-language-descriptors.md), settled
in the grilling of 2026-09-29. The terms are in
[CONTEXT.md](../../../CONTEXT.md), which each stage updates as its piece lands.
Who this is for matters to every brief: **people installing Verkstead
themselves**, on a machine that is theirs alone, building in languages the
maintainer does not. Nothing the maintainer does day to day will notice a
broken Maven variable, which is why the proofs are what they are.

## What was decided, in one place

- **A descriptor is data; behaviour is a built-in capability it names.**
  Rejected: commands in YAML, descriptors compiled in, a trait per language.
- **Built-ins are embedded YAML** in the grammar an installer writes.
- **One map keyed by language**, merged key by key, `null` taking a variable
  out, the Sandbox's own variables refused. `enabled` and `size` are keys of
  the same entry and the only ones the settings page writes.
  `rust_build_cache` is still read as Rust's.
- **A descriptor that does not load falls back to the built-in of that name**,
  and turns the language off only where there is none — so an installer's typo
  costs them their override rather than the cache they had.
- **On by default, set for every session, on all three platforms.**
- **Everything each ecosystem can cache** — downloads and compiled output.
- **C++ through CMake's launcher variables only.** `CC` and `CXX` are left
  alone.
- **The Compile Server starts wherever an sccache language is on**, rather than
  by detection, which is for warnings alone: a manifest that is not at the root
  would otherwise leave a session's own sccache client starting a server inside
  its Sandbox, which is the hazard the server exists to remove.
- **Gradle's daemon is off in a session**; a daemon of Verkstead's own is the
  last stage and opens with a spike.
- **Eviction by whole units**, on a timer, while no session runs. `10G` each,
  Rust's sccache at `30G`. Disk use and a Clear on the page.
- **Accepted:** one session can plant a package another installs.
- **Proven** by environment assertions everywhere and a real install per tool
  in a Sandbox on Linux.

Each stage is one feature: one branch, one review unit. Task chunkings inside
the briefs are provisional — re-grounded against the codebase when the stage
starts.

01 comes first and everything stands on it. 02, 03, 04 and 05 depend on 01 and
not on each other, so they may go in any order; 05 is worth more the more of
the others have landed, because it is their stores it bounds. 06 needs 04.

## Stages

- [x] 01: Language descriptors — [brief](01-language-descriptors.md)
- [ ] 02: Package stores — [brief](02-package-stores.md)
- [ ] 03: C++ through the Compile Server — [brief](03-cpp-through-the-compile-server.md)
- [ ] 04: The JVM — [brief](04-the-jvm.md)
- [ ] 05: Sizes and eviction — [brief](05-sizes-and-eviction.md)
- [ ] 06: A Gradle daemon of Verkstead's own — [brief](06-a-gradle-daemon-of-verksteads-own.md)
