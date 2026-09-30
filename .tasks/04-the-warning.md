# 04. The uncached warning for any sccache language

## What to build

The setup card's warning that compiles won't be cached is Rust's alone today.
`ConversationView::compiles_uncached` is computed in `ui.rs` from Rust's
descriptor being enabled and `build_cache::builds_rust` detecting a
`Cargo.toml`. Extend it so it fires for **any switched-on language that names
the `sccache` capability and is detected in the Repo**. That is Rust, and C/C++
through the `detect` list task 01 gave it. The other two conditions are
unchanged: no sccache on the server, and a platform that compiles through one.
Replace the Rust-only helper with a question asked of the capability rather
than of a language's name.

The copy stops being Rust's too. The setup card's `UncachedCompiles` note says
*crate downloads* and its comments say *a Rust repository*. The settings page's
`uncompiled()` sentence and the comments beside it should be checked for the
same. Rewrite them so they read right for a CMake project, where there are no
downloads to speak of, as well as for a Cargo workspace. Keep saying what fixes
it: install sccache where the server can see it.

## Acceptance criteria

- [ ] On a server with no sccache, the card warns for a Repo whose only manifest
      is a `CMakeLists.txt` at its root, and for one with a
      `native/CMakeLists.txt`, while C/C++ is on.
- [ ] It does not warn for that Repo when C/C++ is switched off, and it still
      warns for a Cargo workspace exactly as before.
- [ ] Neither the card's note nor the settings page says *crate* or assumes Rust.
