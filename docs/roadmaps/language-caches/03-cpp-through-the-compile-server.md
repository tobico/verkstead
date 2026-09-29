# 03. C++ through the Compile Server

## Goal

A session building a CMake project compiles its C and C++ through the one
**Compile Server** the machine already runs for Rust, so an object compiled in
one Conversation is a cache hit in the next. On a machine with no sccache the
setup card says, for a Repo that builds C++, what it already says for one that
builds Rust.

## Decisions in force

From [ADR-0021](../../adr/0021-language-descriptors.md).

- **CMake's compiler-launcher variables, and nothing else.** Setting `CC` and
  `CXX` was asked for, and then withdrawn once its reach was laid out: those
  two change builds that are not C++ projects — a Rust crate with a C build
  script, a Python or Node native extension, Go with cgo — and beside the
  launchers they can wrap one compile twice. Do not reopen it without a new
  reason.
- **What is not covered is documented rather than worked around.** Meson finds
  an sccache on the `PATH` for itself, which on the two Unixes it is. Plain
  Makefiles and Bazel get nothing.
- **sccache only.** vcpkg and conan package caches were offered and turned
  down.
- **The capability is the one Rust's descriptor names.** One Compile Server,
  one store, one size — C++ is a second descriptor switching on the same
  capability, not a second server.
- **The Compile Server starts for a Repo that builds C++**, by the
  descriptor's detection, as it starts for one that builds Rust.
- **The uncached warning extends** to any language that compiles through
  sccache.
- **Proven by a real CMake build** in two Sandboxes on Linux, the second one
  asserted to have hit the cache.

## Proposed tasks (provisional)

1. **The descriptor.** C++ as a built-in, naming the sccache capability and
   setting the launchers. AC: the launchers name the sccache at the path a
   session finds it; with no sccache on the server they are not set at all.
2. **Starting the Compile Server by detection.** AC: a Repo detected as C++
   starts it; a Repo that is neither Rust nor C++ does not; Rust's behaviour
   is unchanged.
3. **The compiler inside the Compile Server's Sandbox.** The server runs the
   compiler, so its own surface has to reach it. AC: a real CMake build in a
   session compiles; a second session's build is served from the cache.
4. **The warning.** AC: the setup card warns for a C++ Repo on a server with
   no sccache and the language on; the settings page's sentence no longer says
   *crate*.
5. **The docs.** What is covered, what is not, and why `CC` is left alone.

## Re-verify at start

- Assumes stage 01 landed with capabilities named by descriptors, and that
  the Compile Server is started from `sessions.rs` on a detection question.
- Check which CMake versions read the launchers from the environment; the
  grilling took it to be 3.17 and later.
- Check that the Compile Server's Sandbox can reach the C and C++ compilers a
  session would run — it was built to reach `rustc` and the rustup home, and a
  compiler from a project's dev shell may be somewhere else entirely.
- Check what detection means for C++, which has no one manifest: a
  `CMakeLists.txt` at the root is the obvious answer and only covers what this
  stage covers.
- Check the Windows arm: whether sccache there wraps the compiler a CMake
  project on that platform uses.
