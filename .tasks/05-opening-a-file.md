# 05. Opening a file

## What to build

A file pressed in the tree opens as a tab beside the terminals.

**The read endpoint carries a version and says what kind of thing it read.**
The version is a hash of the bytes, and it is what a write will later name
itself as being over (ADR-0019, *Versioned reads, and a stale write is
refused*). The kind is one of four: text, an image, a binary it will not send,
or a file over the size cap. An image is previewed in its tab; a binary or an
oversized file is drawn as the line saying why rather than as content. A file
in a read-only root opens read-only and takes no typing.

**Text is drawn plainly here.** Monaco is the next task: what this one builds
is the tab, the kinds and the buffer behind them, so that the editor is swapped
in against something that already works. The same file opened twice is one
buffer under two views — there is one group in this stage, so that means one
tab, turned to rather than opened again.

The refusals stay sentences in the body, beside the ones the roots and folders
answer with. Wire types and the vitest suite's golden fixtures are regenerated
from the real endpoint; nothing on the viewer's side is hand-written.

## Acceptance criteria

- [ ] Pressing a text file in the tree opens a tab showing its contents, and
      the read carries a version.
- [ ] A PNG shows as a picture; a binary and a file over the cap each show the
      line saying why.
- [ ] A file in a read-only companion opens read-only and takes no typing.
- [ ] Pressing the same file twice turns to the tab it already has rather than
      opening a second.
