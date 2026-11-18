# 03. The size field

## What to build

Every Language plugin declares a settings field `size` of kind `size`, drawn in
its pane by the pane stage 01 built, with the language's default as its
placeholder: `30G` for Rust & C/C++ (what its sccache has always been started
at), `10G` for the rest. One field per plugin, so Rust & C/C++ has one size —
one Compile Server, one store, one size, as today.

The value is the record's, so the cluster's. `{size}` in a session's variables
(`SCCACHE_CACHE_SIZE`), the Compile Server's start and the store trimming all
read it from the plugin's record now, with the default where nothing is set. A
value that is not a size in sccache's grammar is refused at the save with the
reason at the field, as the Sandbox page refused it.

The Sandbox page's size fields go.

**The take-up of the size**, once, beside task 02's take-up of `enabled`:
`languages.<name>.size` and `rust_build_cache.size` (read as Rust's, the map
winning where both are written) are read and not written again. **The larger
wins** across devices, so the take-up never shrinks a cache somebody was using;
as with the tick, a value a human saved on the pane since is not overwritten by
a device upgrading later. A size a hand-edit left unreadable is taken up as
nothing, so the default stands.

## Acceptance criteria

- [ ] A size saved on Rust & C/C++'s pane is what the next session's `SCCACHE_CACHE_SIZE` reads, and what the trimming holds the cargo store to; one saved on Go's pane is what Go's stores are trimmed to.
- [ ] `1.5g` is refused with the reason and nothing is written.
- [ ] Two devices whose `config.yaml` said 20G and 40G for a language take up as 40G; a `rust_build_cache.size` from an old `config.yaml` still takes up as Rust's; the Sandbox page has no size fields.
