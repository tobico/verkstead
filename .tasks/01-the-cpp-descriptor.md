# 01. The C/C++ descriptor

## What to build

Add a built-in descriptor to the embedded `languages.yaml`. It is keyed `cpp`,
labelled **C/C++** on the settings page, and names the `sccache` capability.
The capability's variables are **only** `CMAKE_C_COMPILER_LAUNCHER` and
`CMAKE_CXX_COMPILER_LAUNCHER`, both `"{sccache}"`.

- **Leave out `SCCACHE_DIR` and `SCCACHE_CACHE_SIZE`.** They are Rust's
  descriptor's, and a variable set by two languages is pushed onto the session
  twice, which nothing in the loader resolves. The session's client never needs
  them while Verkstead's Compile Server is up. What the server reads is set by
  the server itself, in `build_cache.rs`.
- **`CC` and `CXX` are never set** (ADR-0021, a decision in force).
- `detect` holds `CMakeLists.txt`, `native/CMakeLists.txt`,
  `cpp/CMakeLists.txt` and `meson.build`. `Descriptor::detected` joins each
  entry onto the Repo's root, so a relative path works as plain data. Meson
  isn't covered by the launchers, but it finds an sccache on the `PATH` for
  itself, and without one on the server it is just as uncached. So it belongs
  in the warning's detection.
- Comment the entry the way the other built-ins are commented. Say why these
  two variables and not `CC`. Say that the launchers are read from the
  environment by CMake 3.17 and later, by the Makefile and Ninja generators
  only, and only when a build directory is first configured.

**The Compile Server's start needs no change here.** Stage 01 put it on the
switch: `Languages::wanting(SCCACHE)` answers for the first switched-on
language naming the capability, so C++ being on with Rust off starts it.
Prove it rather than rebuild it.

**One Compile Server, one size.** This was decided when the plan was put to
the human. The settings page draws the size field **only on the language that
sizes the Compile Server**, which is the first switched-on one naming the
capability (the same answer `wanting` gives). Any other language naming the
capability says instead that it compiles through the same cache as that
language, naming it by label. If Rust is off and C++ is on, the field is on
C++. The size key is still sent back for every entry, as `LanguageView.size`
says. Only whether a field is drawn changes.

## Acceptance criteria

- [ ] On a machine with an sccache, a session is given both launchers naming
      the sccache at the path the session reaches it by. This is asserted on
      all three platforms, the way Rust's `RUSTC_WRAPPER` is.
- [ ] On a machine with no sccache, neither launcher is set, and `CC`/`CXX` are
      set in no case.
- [ ] With Rust off and C/C++ on, the Compile Server is wanted, at C/C++'s
      size. With both on, Rust's variables and size are exactly what they were
      before this descriptor existed.
- [ ] The settings page draws a size field on exactly one of the two languages
      and a "shares the Compile Server with …" line on the other, and switching
      Rust off moves the field to C/C++.
