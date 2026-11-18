# 03. Running a wasm component

## What to build

wasmtime 50 in the server, with the component model and Cranelift. A manifest's
`component:` is instantiated under the capabilities the manifest asks for and
nothing else:

- **Filesystem paths** become preopens, read-only or read-write as declared.
- **Environment variables** are the named ones, with the server's values.
- **Outbound hosts** are reachable through WASI HTTP, with a request to any
  other host refused. Raw sockets are not granted at all.
- Nothing else: no inherited stdio beyond what is logged, no clocks or
  randomness beyond WASI's defaults.

The WIT package declares a base world every plugin component may target, and
one world per Category (six), each with nothing in it yet. Later stages fill
them. Bindings are generated with wasmtime's `bindgen!`.

`verkstead plugin check` instantiates the component, so a component that traps
at instantiation, or does not satisfy the base world, is a refusal the check
prints. The server-side loader returns the same failures as **trouble**, a value
a later task draws on a row, and never a panic or an error that stops the
server.

Test fixtures are written as WAT and assembled at test time (the `wat` crate),
so the suite needs no wasm toolchain: one component that exports nothing, one
that reads a path it was not granted, one that reads a path it was granted, and
one that traps in its start function. Record in the memo or the PR the size
wasmtime adds to the release binary. Planning measured about 17.6 MB on wasmtime
48 against ADR-0030's 10 to 20 MB.

wasmtime has its own `wasmtime::Error`, which is not `std::error::Error`.
Convert it at the loader's boundary.

## Acceptance criteria

- [ ] A component exporting nothing loads, and `plugin check` on its directory
      passes.
- [ ] A component reading a path it was not granted cannot read it, and one
      reading a granted path can.
- [ ] A component that traps at instantiation is trouble, refused by
      `plugin check` with that said, and not a crash.
- [ ] An outbound request to a host the manifest did not name is refused.
