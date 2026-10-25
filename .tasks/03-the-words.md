# 03. The words

## What to build

Everything a Mac user, or the human releasing, reads about the Mac download
says it is signed and notarized. Read `docs/copy.md` and `CONTEXT.md` first.

- **The README's Mac section** loses *"The app is unsigned"* and the three
  *Open Anyway* steps. What a downloader still meets — drag into
  Applications, and the one *downloaded from the Internet* confirmation on the
  first open — is what it says instead. The Windows paragraph is not touched:
  Windows signing was declined.
- **`docs/releasing.md`**: the Mac leg's prose says it signs, notarizes,
  staples and packs the zip, and what it asserts of the dmg and the zip
  (tasks 01 and 02), and that a release with no Mac secrets stops there. The
  *After the run* dmg step (item 4) drops the walk through Privacy & Security:
  downloaded in a browser, it opens after the one confirmation and nothing
  more — an *Open Anyway* there is a failed release. Item 8's reference to the
  README's `## Updating` section stays, it being stage 04's.
- **`desktop/electron-builder.yml`'s comments** that say the app is unsigned,
  that there is no Developer ID, or why the hardened runtime is off, say what
  is now true. Comments elsewhere naming the Mac app unsigned (the Mac leg in
  `.github/workflows/release.yml`) likewise; Windows ones stay.
- **ADR-0020** gains an *Amended* note: the Mac app is Developer ID signed and
  notarized, by ADR-0027; Windows stays unsigned.

## Acceptance criteria

- [ ] No *Open Anyway*, and no "unsigned" about the Mac app, anywhere a Mac user reads or in `docs/releasing.md`.
- [ ] The checklist's dmg step says what a signed download shows and that an *Open Anyway* is a failed release.
- [ ] ADR-0020 carries the amendment, pointing at ADR-0027.
