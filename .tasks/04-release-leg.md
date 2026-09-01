# 04. The release leg

## What to build

The macOS desktop job on the plumbing stage 03 laid, so that a tag carries the
dmg.

**One `macos-15` job**, beside the AppImage's rather than a row in the CLI
matrix, for the reasons that leg gives about itself: it builds a bundle rather
than a bare binary and nothing about it is that matrix's shape. It builds both
Apple targets itself and `lipo`s them — an Apple silicon runner carries the
x86_64 SDK, so one job needs no second runner and no joining job — fetches the
viewer artifact the way every other leg does, runs the bundle script, and
asserts what it built the way `desktop-linux` asserts its own: the app runs, the
viewer is really inside it rather than a 503, and the tray comes up.

**`publish` gains one line and nothing else.** Stage 03 wrote that job to carry
any desktop artifact: it fetches `desktop-*`, keeps those out of the CLI count,
and names its assets in one list which is the only place a desktop asset name is
written down. So this is an artifact uploaded under the `desktop-` prefix and
the dmg's own name added to that list — the job itself is not touched.

The install story is not this task's: what is true about sessions on a Mac is
still moving, and it is written once at the end of the stage.

## Acceptance criteria

- [ ] A rehearsal run started by hand builds the dmg, asserts the app runs out
      of it and that the viewer is inside it, and reaches `publish` with the dmg
      named among the desktop assets
- [ ] The CLI binaries are still counted as a set of their own, so a missing one
      still fails the run
- [ ] Nothing else in `publish` changes
