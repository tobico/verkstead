# 01. Packaging assets and the AppImage

## What to build

The Linux desktop artifact, from the artwork the repository already has to a
single file that runs the tray on a machine that has never heard of this
project.

Two halves, in one slice because the first exists only to be consumed by the
second. **The packaging assets**: the `.desktop` entry naming the app id
`net.tobico.Verkstead`, and the sized launcher icons a desktop's own menu draws
from. They are generated from `assets/icons/verkstead-hammer.png` by a script
beside `tools/generate-icons.sh` — the same rule as the viewer's icons: one
piece of artwork, everything else is output, committed so a build needs nothing
extra, never hand-edited. They land in a `packaging/` directory at the
repository root, **not** under `assets/`, which vite serves whole as the
viewer's `publicDir` and which is therefore embedded in every binary including
the headless CLI. Stages 04 and 05 put their own `.icns` and `.ico` beside
these. Note that these are the *launcher's* icons: the icon the running panel
draws is compiled into the desktop binary already and is not this.

**The bundle**: a script that builds the desktop binary for x86_64 Linux and
wraps it, the packaging assets and the system libraries it links into
`Verkstead-x86_64.AppImage` — the format's own `Name-arch` convention rather
than the release's `verkstead-linux-x64` scheme. The binary is dynamic, so what
must ride inside is GTK3 and the appindicator library the tray is drawn over,
whichever bundler is used to gather them. x86_64 only; an arm64 Linux user has
the bare CLI.

The script is the thing CI will call in the next task, so it takes its inputs
from the working tree and leaves the artifact where a caller can find it, rather
than assuming either a developer's machine or a runner.

## Acceptance criteria

- [ ] The generation script rewrites the whole packaging set from the one piece
      of artwork, and a second run leaves it byte-identical; the desktop entry
      passes `desktop-file-validate`.
- [ ] Nothing desktop-only reaches the viewer build or the CLI binary — the
      packaging directory is outside what `publicDir` copies, and a built viewer
      serves none of it.
- [ ] One command produces `Verkstead-x86_64.AppImage`, in the dev shell and on
      a bare ubuntu-24.04 runner alike.
- [ ] That file, run on a stock distribution image carrying no GTK development
      packages, serves the viewer at the address it was given, opens nothing
      under `--no-open`, and exits cleanly — which is the bundling proved rather
      than assumed.
