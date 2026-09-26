# 01. The universal dmg, packed from a checkout

## What to build

`electron-builder.yml` gains the `mac:` section it was written to have room for,
beside the `linux:` one and under everything both already share. `pnpm run pack`
on a Mac then produces `Verkstead-universal.dmg` — the Electron app, universal,
with the two Mac CLI builds joined into the one sidecar — and that file mounts,
opens its window, puts an icon in the menu bar and serves the workbench.

**First, because nothing after it can be looked at.** `startup.ts` answers
`nowhere` on an unpackaged run, by decision (ADR-0020, Q2): a registration made
from a checkout would name the dev shell's Electron in the nix store. So Launch
on Startup, the launch agent take-over and a login start are all unreachable
from `pnpm start`, and there has to be a packed app before the tasks that prove
them have anything to run.

**The CLI is `lipo`'d by the pack rather than by the leg** (Set 889 Q3a), so a
developer packs the way the release leg does. `scripts/pack.mjs` takes one path
today, in `VERKSTEAD_CLI` or as its first argument; on a Mac it takes two and
joins them into the single staged `cli/verkstead` that `electron-builder.yml`
already names. How the two are said — two positional paths, or a pair of
variables beside the existing one — is this task's to settle, but the existing
single-path spelling has to go on working on the other two platforms.

**One universal file is also what a universal pack requires.** electron-builder
builds the x64 app and the arm64 app and hands the two to `@electron/universal`,
which refuses a file that differs between them unless a `x64ArchFiles` pattern
excuses it. A universal sidecar is byte-identical in both halves by
construction, so there is nothing to excuse. The `lipo -archs` check the Rust
script made of its own output carries over: a `lipo` given one input writes a
perfectly good single-architecture file, and the Mac that could not run it is
the one nobody testing this has.

**Ad-hoc signed, and that is not the signing this roadmap decided against.**
electron-builder's `identity: null` skips code signing outright, and an
Apple-silicon Mac refuses to execute a Mach-O carrying no signature at all — so
the app would not start on half the Macs it is built for. `identity: "-"` is
ad-hoc signing through `@electron/osx-sign`, which is exactly what
`tools/build-macos-dmg.sh` did by hand with `codesign --sign -`. It buys nothing
from Gatekeeper: there is no Developer ID, the download is unsigned in the sense
the adoption documents mean, and the three steps they describe stay (Set 848
Q16a).

**And `hardenedRuntime` has to go off beside it.** It defaults on, and
electron-builder's own schema warns that ad-hoc signing with it enforces library
validation, which rejects the pre-signed Electron framework for carrying a
different Team ID — an app that packs and then will not launch. Nothing here is
notarized, which is the only thing the hardened runtime is required for.

**The icon is the committed `.icns`, staged rather than pointed at.**
`packaging/net.tobico.Verkstead.icns` is what `tools/generate-packaging.sh`
writes from the same artwork the Linux launcher icons come from, and staging it
the way the hicolor set is staged keeps that script the only thing that writes
artwork. The Finder icon is the one thing about a bundle that nothing running
the app can tell you about — v0.1.2 shipped an icns under a `CFBundleIconFile`
naming no file and drew the generic icon on every Mac that downloaded it — so it
is read back off what was packed rather than trusted.

**No `LSUIElement`.** The key is what made the Rust bundle a menu-bar app with no
Dock tile, and ADR-0020 retires it: the app is a regular Dock app now.
electron-builder writes no such key, so this is a thing to confirm in the packed
`Info.plist` rather than a thing to set — and the confirmation is worth making,
because it is the whole of what task 03 rests on.

**The artifact's name is written down rather than defaulted**, for the reason
the AppImage's is: `publish` names `Verkstead-universal.dmg`, the adoption
documents spell it, and electron-builder's own pattern would put the version in
the middle of both. What `LSMinimumSystemVersion` the packed bundle claims is
Electron's floor rather than rustc's 11.0, and it is read back off the artifact
rather than stated — it is the promise the adoption documents make to a
downloader, and task 06 writes whatever this reads.

## Acceptance criteria

- [ ] A dmg packed from a checkout on a Mac mounts, and the app inside it opens
      its window, puts an icon in the menu bar and serves the workbench — its
      own log under `~/Library/Logs/Verkstead` saying both.
- [ ] The `verkstead` in the mounted app's resources holds both `arm64` and
      `x86_64`, and answers `ask --help` and `guide` run by path out of the
      mount.
- [ ] The app starts on Apple silicon, `codesign --verify --strict` passes over
      the mounted bundle, Finder draws the real icon, and the packed
      `Info.plist` has no `LSUIElement` — so there is a Dock tile.
- [ ] `pnpm run pack` still takes one path and packs an AppImage on Linux
      unchanged.
