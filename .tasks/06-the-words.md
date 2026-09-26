# 06. The words

## What to build

The Mac paragraphs, rewritten for the app that now ships. What they describe
today is a menu-bar app with a four-item menu, built by a shell script, whose
bundle holds one binary and a launcher — and none of that is true any more.

**`docs/adoption.md`, *The desktop app, on a Mac***. The dmg holds the Electron
app with the released `verkstead` beside it in its own resources directory,
rather than one binary and a launcher script supplying a verb — so the paragraph
about the launcher goes and the one the Linux section already has about the
sidecar being the released artifact comes in. It is a regular Dock app with a
window: closing leaves it in the Dock, a Dock click brings it back, Cmd+Q quits.
The window has no title bar and the traffic lights stand in the head. The
four-item menu becomes the tray's three plus the **Desktop** section of the
settings page, which is where the menu bar icon, **View Logs** and **Launch on
Startup** live. Launch on Startup is the login item macOS lists in System
Settings rather than a plist — so the paragraph saying the checkbox cannot see
that list is rewritten or dropped, the registration now *being* that list — and
an upgrade from the tray app takes the old launch agent over once, silently.
`--no-open` and `--data-dir` go: the app takes no flags, and
`VERKSTEAD_DATA_DIR` is how the Data Directory is moved, exactly as the Linux
section says.

**The Gatekeeper steps stay** (Set 848 Q16a), word for word if they still
read true: the app is ad-hoc signed so that Apple silicon will execute it, which
is not a Developer ID and buys nothing from Gatekeeper. Check the oldest macOS
the packed bundle claims against what the section promises — it is Electron's
floor now rather than rustc's 11.0, and task 01 reads it off the artifact.

**`docs/releasing.md`**: the `desktop-macos` leg described for what it does —
waits on the CLI matrix, downloads both Mac binaries, joins them and packs — and
not for a script that is gone.

**`docs/development.md`**: the build list drops the `tools/build-macos-dmg.sh`
line, and the paragraph describing the dmg describes the packed Electron app.
The `LSUIElement` that made it a menu-bar app with no Dock tile is among what
goes, along with the launcher script and the by-hand ad-hoc signing.

**`README.md`'s Mac line**, and a sweep of **`CONTEXT.md`** for the launcher,
the plist and anything saying the Mac app is menu-bar-only.

**What stays.** ADR-0012 and the roadmap's own briefs are history and are not
rewritten. `verkstead desktop` still exists until stage 08 retires it — what
must not name it is a paragraph telling a human what to run on a Mac.

## Acceptance criteria

- [ ] Nothing outside ADR-0012, ADR-0020 and the roadmap briefs names
      `Verkstead-launcher`, `tools/build-macos-dmg.sh` or `LSUIElement`, and no
      Mac paragraph names `verkstead desktop`.
- [ ] The Mac section describes a Dock app with a window, a Desktop page and a
      login item, with the Gatekeeper steps kept and the system version it
      promises matching what the packed bundle claims.
- [ ] `cargo test`, the viewer and desktop suites, `actionlint` and `nix flake
      check` are green.
