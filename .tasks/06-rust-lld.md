# 06. Try rust-lld

## What to build

rustc ships `rust-lld` in its sysroot, and on MSVC targets it links faster
than `link.exe`. Its gain on this workspace is unmeasured, so this task is an
experiment with a rule for what happens afterwards: a commit of its own that
links the Windows build with `rust-lld`, measured by CI, kept if the ledger
shows a saving and reverted if it does not.

If it stays, it stays for everyone: the setting goes in the checked-in
`.cargo/config.toml` under the MSVC target, so a Windows developer links
faster too, and the comment there says what it is and what it saved. The
sysroot's `rust-lld` is not on the `PATH`; find the spelling rustc 1.95
accepts on stable for this target — `linker = "rust-lld"` resolves to the
sysroot's copy where rustc knows to look, and a `linker-flavor` or a full path
is the fallback — and prove it links the whole workspace, test binaries
included, on a Windows machine before it goes to CI. The sessions suite's
`a_rust_session_compiles_through_the_compile_server` runs a real cargo build
inside a session on the runner, so the setting reaches it too; it must still
pass.

Record the build time of each leg in the ledger either way, and say whether
the linker stayed. A revert is a commit too, with the figures in its message.

## Acceptance criteria

- [ ] A commit links the Windows build with `rust-lld` from the checked-in cargo config, every leg is green with it, and the ledger carries both legs' build times from that run
- [ ] The ledger says whether it stayed, and if not the branch no longer carries it
