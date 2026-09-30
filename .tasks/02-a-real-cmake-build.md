# 02. A real CMake build through the Compile Server

## What to build

A proof that a real CMake project compiles inside a session's Sandbox on Linux
through Verkstead's Compile Server. The proof follows the pattern of
`tests/package_stores.rs`, a real tool run in a real Sandbox and skipped where
the tool is missing. Add `cmake` and `ninja` to the dev shell and to Linux CI
so it runs there rather than being skipped.

Re-grounding found that the Compile Server's Sandbox stands on the same system
floor as a session's (`/nix`, `/usr` and the rest of `SYSTEM`). It is handed
the same `PATH` through `sandbox::reaching`. So a compiler a session can run,
whether from the machine or from a dev shell under `/nix`, is one the server can
run too. The server receives the client's environment for each compile, which
is what a nix `cc-wrapper` reads its flags from. Nothing is expected to need
widening. If something does, widen it here, in the Compile Server's own
description, and say why. The one known gap is a compiler behind a
Conversation's own binds, which the server does not get. It is documented in
task 05 rather than worked around.

The project should be small but real: a `CMakeLists.txt` with one C and one C++
source and a header under an `include/` directory, so `-I` is on the command
line. Configure it with the Ninja generator into a build directory **inside the
Worktree**, then build it. Then remove the build directory and configure and
build again in the same Worktree.

Read the hits and misses off the Compile Server itself: `sccache --show-stats`
through the same client, or its JSON form. Don't infer them from timing.

## Acceptance criteria

- [ ] A CMake build in a Linux session's Sandbox compiles, with every C and C++
      compile going through the Compile Server. The server's stats show the
      compile requests, and no sccache server process was started inside the
      session's Sandbox.
- [ ] The clean rebuild in the same Worktree is served entirely from the cache,
      asserted from the server's stats.
- [ ] `cmake` and `ninja` are in the dev shell and in Linux CI, and the proof
      skips with a message rather than failing where they are absent.
- [ ] Rust's existing Compile Server proofs pass unchanged.
