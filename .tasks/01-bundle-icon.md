# 01. The bundle draws its icon

## What to build

`Verkstead.app` shows macOS's generic app icon in Finder, the Dock and Get
Info, while Quick Look on the `.icns` inside it draws the hammer. The file is
sound — `iconutil` extracts every slot — and the fault is how `Info.plist`
names it: `CFBundleIconFile` is `net.tobico.Verkstead`, with no extension.
macOS appends `.icns` only to a value with no extension, and a value with dots
in it reads as having one, so the system looks for a file called exactly
`net.tobico.Verkstead`, finds none, and draws the placeholder. Confirmed on
macOS 15.5: a copy of the app with the value changed to
`net.tobico.Verkstead.icns` draws the hammer.

The bundle script writes the extension into the value. Then the dmg leg of
the release workflow, which mounts the image and asserts the app inside it,
gains an assertion that the icon resolves: it reads `CFBundleIconFile` off the
mounted bundle's plist, requires a file of exactly that name in
`Contents/Resources`, and has `iconutil` convert it to an iconset — a value
that resolves to nothing, or an icns macOS cannot read, fails the leg rather
than shipping. The check belongs beside the leg's other bundle assertions (the
plist lints, the signature verifies), not inside the script that wrote the
plist: the point is a Mac reading back what was written.

`docs/development.md`'s description of the dmg names the plist keys; keep it
true.

## Acceptance criteria

- [ ] `Verkstead.app` dragged from a freshly built dmg shows the hammer in
      Finder, in Get Info and in the dialog macOS draws for an unsigned app.
- [ ] The dmg leg fails on a `CFBundleIconFile` that names no file in
      `Contents/Resources`, and on an icns `iconutil` refuses; it passes on
      the bundle the script now writes.
- [ ] The committed `.icns` and `tools/generate-packaging.sh` are untouched:
      the file was never the fault.
