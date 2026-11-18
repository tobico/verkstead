# 04. Built-in components embedded in every binary

## What to build

A component is platform-neutral bytes, so the built-ins are built once and
embedded everywhere, the way the viewer is embedded today. This task lands that
path with a **fixture**, before a harness depends on it in stage 04 of the
roadmap.

- A fixture component crate in the workspace (excluded from the native build)
  that builds for `wasm32-wasip2` against the base world and exports nothing,
  with a manifest beside it under a fixed uuid.
- The release workflow builds every bundled component **once** for
  `wasm32-wasip2` in a job of its own. The musl x86_64 and aarch64 Linux legs,
  both Mac legs before the lipo, and the Windows leg each embed the same
  artefact. CI's build does the same, so the test suite runs against embedded
  bytes.
- The server loads the embedded built-ins at start through the same loader as an
  installed plugin. A built-in that fails to load is trouble, and the server
  starts anyway.
- A local build without the artefact still works. Settle how (for example,
  building the component in a build script when the target is present, else a
  clear error naming the command), and say it in `docs/development.md`.

**The fixture is not listed** on the Plugins page (Set 1772, Q4). It is a proof
of the path, not a plugin anyone uses, and the Built-in group only appears once
it has a row, which is stage 02's Languages.

The brief's re-verify item is this task's acceptance: wasmtime has to build for
**every release target**. Prove it on a branch run of the release legs (the
memories say how to run a Mac leg from here). **A target it cannot build for is
a decision to bring back to the human** with `verkstead ask`, not a workaround
to find.

## Acceptance criteria

- [ ] The release workflow builds the fixture once for `wasm32-wasip2`, and all
      five release legs (x86_64 and aarch64 musl, x86_64 and aarch64 Darwin
      lipo-merged, x86_64 Windows MSVC) build and embed it. This is proven by a
      workflow run, not assumed.
- [ ] The server loads the embedded fixture at start (tested), and a built-in
      whose bytes fail to instantiate leaves the server running with trouble
      recorded.
- [ ] The size wasmtime adds to the musl binary is recorded against the ADR's 10
      to 20 MB.
