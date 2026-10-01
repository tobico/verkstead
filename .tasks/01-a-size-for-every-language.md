# 01. A size for every language

## What to build

Today only the language whose store sccache bounds (Rust) draws a size field on
the **Language support** pane, every language's default is `30G`
(`build_cache::SIZE`, used by `Descriptor::size`), and nothing in the server
parses a size — the human's word is passed verbatim into `{size}`.

After this task every language **with a store of its own** draws a size field
under its checkbox, with its default as the placeholder: **`10G`** where nobody
has said, and **`30G` for Rust**, whose sccache keeps the size it has today.
The default is a release's, not a machine's, so a save that typed nothing still
writes no `size` into `config.yaml` (the page writes `enabled` and `size` only,
and a blank size is *nothing configured*, as now). How a built-in carries its
own default — a key in the embedded grammar that is not the machine's `size`,
or a constant — is this task's to settle; an installer's own descriptor gets
the `10G`.

**C/C++ draws no field.** It names the `sccache` capability and has no store of
its own; the one Compile Server is sized by the first descriptor naming the
capability (Rust, on or off), exactly as `Languages::wanting` and the pane's
`sizer` say today. Keep that.

**Verkstead now reads a size itself**, because the sweep (task 04) has to
compare bytes against it. Read it in sccache's own grammar — a number with a
`K`/`M`/`G`/`T` suffix, binary multiples — so one word means the same to
sccache and to Verkstead. `{size}` still reaches a self-evicting tool as the
human's own word, through its own variable (`SCCACHE_CACHE_SIZE`). A size the
server cannot read is **refused at save** with the reason on the page; one
written by hand in `config.yaml` that cannot be read falls back to that
language's default, and the pane says why, the way an unread entry already does.
Update the comments in `languages.yaml`, `languages.rs` and the pane that say
"absent is 30G" and "nothing here parses it".

## Acceptance criteria

- [ ] Every language with a store draws a size field; the placeholder is `30G` for Rust and `10G` for every other built-in and for an installer's own descriptor; C/C++ draws none.
- [ ] A save with nothing typed writes no `size`; a typed size reaches `SCCACHE_CACHE_SIZE` (Rust) and `{size}` in any descriptor's variable unchanged.
- [ ] An unreadable size is refused at save with a reason the page shows, and an unreadable one in `config.yaml` falls back to the default with the reason on the pane; sizes parse to bytes with sccache's grammar, covered by unit tests.
