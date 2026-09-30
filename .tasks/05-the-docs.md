# 05. The docs

## What to build

Say what C/C++ caching covers and what it doesn't, where an installer and a
future maintainer will look. That means `CONTEXT.md` (the **Compile Server**
and **Descriptor** entries, which name Rust as the only language naming the
capability), the C/C++ entry's comments in `languages.yaml`, and the
installer-facing docs that already describe the Build Cache (`docs/adoption.md`
and the design doc's Build Cache section, whichever say it today).

What to say:

- **Covered:** CMake 3.17 and later, with the Makefile or Ninja generator, reads
  `CMAKE_C_COMPILER_LAUNCHER` / `CMAKE_CXX_COMPILER_LAUNCHER` from the
  environment when a build directory is first configured. A build directory
  configured before C/C++ was switched on keeps what it cached.
- **Meson** finds the sccache on the `PATH` for itself.
- **Not covered:** plain Makefiles, Bazel, and CMake's **Visual Studio** and
  **Xcode** generators, which ignore the launchers. Visual Studio is CMake's
  default on Windows, so a Windows Repo is cached only when it picks Ninja. MSVC
  debug info in `/Zi` form isn't cacheable by sccache either (`/Z7` is).
- **Why `CC`/`CXX` are left alone:** they reach builds that are not C++ projects
  at all (a Rust crate's C build script, a native Node or Python extension, Go
  with cgo), and beside the launchers they can wrap one compile twice.
- **Base directories:** a second Conversation hits the cache because the Compile
  Server is told every Worktree. A new one is taken in only while no session is
  running, so a busy machine misses until it's quiet. It needs an sccache new
  enough to honour `SCCACHE_BASEDIRS`; task 03 records the version. An object
  served from another Conversation's compile carries that Worktree's path in
  its debug info.
- **Where a build may go:** the Compile Server writes compiled output where the
  compile names it. A build directory outside the Worktree, such as the
  session's own `/tmp`, is one the server can't reach, so that compile fails
  rather than just missing the cache. The same is true of Rust's `target/`. A
  compiler reached only through a Conversation's own binds is one the server
  can't run either.

## Acceptance criteria

- [ ] `CONTEXT.md` names C/C++ beside Rust wherever it says which languages
      compile through the Compile Server.
- [ ] The installer-facing docs say what is covered, what isn't, and why `CC`
      is left alone, in the project's own vocabulary.
- [ ] The build-directory and Conversation-bind limits are written down where
      someone hitting a failed compile would look.
