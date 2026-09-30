# C++ through the Compile Server

A session building a CMake project compiles its C and C++ through the one
**Compile Server** the machine already runs, so an object compiled in one
Conversation is a cache hit in the next. C++ is a second built-in descriptor
naming the same `sccache` capability as Rust's, and it sets CMake's two
compiler-launcher variables and nothing else. `CC` and `CXX` are left alone
(ADR-0021).

Re-grounding found that sccache hashes a C++ compile's absolute paths. Every
Conversation has a Worktree at a path of its own, so without more work a second
Conversation's build would miss every time. The Compile Server is therefore
also told every Worktree as a base directory, which is task 03. On a machine
with no sccache, the setup card warns for a Repo that builds C++ as it already
does for one that builds Rust.

Roadmap stage: [03: C++ through the Compile Server](docs/roadmaps/language-caches/03-cpp-through-the-compile-server.md)

## Tasks

- [x] 01: The C/C++ descriptor — [details](01-the-cpp-descriptor.md)
- [x] 02: A real CMake build through the Compile Server — [details](02-a-real-cmake-build.md)
- [ ] 03: A second Conversation's build hits the cache — [details](03-a-second-conversation-hits.md)
- [ ] 04: The uncached warning for any sccache language — [details](04-the-warning.md)
- [ ] 05: The docs — [details](05-the-docs.md)
