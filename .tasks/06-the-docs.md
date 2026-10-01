# 06. The docs

## What to build

What a person installing Verkstead reads about sizes and eviction, in the
places they already read about languages: `docs/adoption.md`'s **Languages**
section, the grammar's own comments in `crates/server/languages.yaml`, and
`CONTEXT.md` (**Descriptor**, **Build Cache**, **Compile Server**, and a term
for the sweep and the unit if the code settled one).

- **What is evicted**: each built-in store and how it is bounded — by its tool
  (sccache), by unit, or not at all — and the default sizes (`10G`, Rust's
  `30G`, the cargo half held to Rust's size separately from sccache). How an
  installer names a unit for their own descriptor, by depth or by marker, with
  the built-ins as worked examples, and why never by file except a blob.
- **When**: hourly, only while no session or terminal runs, at the first idle
  moment after it is due — and that a machine that never goes idle is never
  swept. Clear, and that it is refused while anything runs.
- **What *oldest* means on the reader's filesystem**: access time where it is
  kept and modification time where it is not, and which a reader likely has —
  Linux's `relatime` default against `noatime` mounts, macOS's APFS, and NTFS,
  whose last-access updates Windows may have switched off — because without
  access times *least recently used* quietly becomes *least recently written*.
  Check each platform's current default rather than writing it from memory.
- **ADR-0021 amended**: its *Eviction is by whole units* section says a unit is
  a directory depth; amend it with what Set 996 decided — a depth or a marker, a
  blob in a content-addressed store whose tool verifies it, the cargo half
  swept, Clear refused while anything runs, a never-idle machine never swept —
  in the way the ADR's stage 04 amendments are written.

## Acceptance criteria

- [ ] `docs/adoption.md` says what is evicted, when, how big, how to name a unit, and what *oldest* means on Linux, macOS and Windows filesystems.
- [ ] ADR-0021 is amended with this stage's decisions, and `CONTEXT.md` and the grammar comments in `languages.yaml` match the code as landed.
