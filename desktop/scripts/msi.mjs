//! The two things the msi cannot be told, written into the WiX project
//! electron-builder generated.
//!
//! **This is the one hook in the pack, and it is here because the msi target has
//! the one gap.** On the version `package.json` pins, that target reads its
//! `template.xml` out of `app-builder-lib`, hands candle a single `project.wxs`
//! and light a single `project.wixobj`: there is no `buildResources` override and
//! nothing a WiX source of our own could be linked in as. What it does have is
//! `msiProjectCreated` — a configuration key naming a file that runs after the
//! project is written and before candle compiles it — so this is where the two
//! things go that no configuration key says (ADR-0020).
//!
//! **The `PATH` entry**, which is half of what a Windows download is for:
//! `verkstead ask`, `verkstead guide` and the rest have to work in a terminal
//! opened after the install. It names [`CLI`] inside the install rather than the
//! install root — the root holds the launcher, `Verkstead.exe`, and Windows
//! resolves a `PATH` lookup without regard to case, so a root on `PATH` would
//! answer `verkstead guide` with a window instead of the Guide. A rename would
//! not fix that either: one NTFS directory does not hold `verkstead.exe` and
//! `Verkstead.exe` both. The entry is the retired `tools/verkstead.wxs`'s own,
//! retargeted — `Part="last"` so it appends to whatever the user's `PATH` already
//! said, `System="no"` so it lands in `HKCU` where a per-user install belongs,
//! and `Permanent="no"` so the uninstall takes it away. It goes *inside* the
//! component the sidecar is, which is what ties the two together: the entry
//! arrives and leaves with the file it names.
//!
//! **And the install directory's name.** electron-builder derives it from
//! `package.json`'s `name`, which would put the app under
//! `%LOCALAPPDATA%\Programs\verkstead-desktop`; what the Rust msi used and what
//! `docs/adoption.md` names is `…\Programs\Verkstead`. The target will use the
//! product name instead, but only for an assisted or a per-machine package, and
//! this one is neither.
//!
//! **It is string surgery on a generated file, so it fails loudly.** A patch that
//! found nothing to change stops the pack rather than quietly leaving an msi with
//! no `PATH` entry in it — that being the one way this breaks on an
//! electron-builder upgrade, and the `desktop-windows` leg's assertions being the
//! other half of the answer. Every patch below matches exactly once or nothing is
//! written.
//!
//! Plain JavaScript and outside `src/`, for the reason `pack.mjs` and `start.mjs`
//! are: it is part of what packs the app rather than part of the app.

import { readFile, writeFile } from "node:fs/promises";

/// What the install directory is called, under `%LOCALAPPDATA%\Programs`.
const DIRECTORY = "Verkstead";

/// Where the sidecar sits inside the install, as its path segments — which is
/// `extraResources` in `electron-builder.yml` and the directory `src/cli.ts`
/// looks in, said a third time because this is what goes on `PATH`.
const CLI = ["resources", "cli"];

/// And what it is called there, which is `pack.mjs`'s answer for this platform.
const EXE = "verkstead.exe";

/// The entry itself. `[APPLICATIONFOLDER]` is the install directory whatever the
/// profile turns out to be, and the rest is the sidecar's directory inside it.
const ENTRY =
  `<Environment Id="UserPath" Name="PATH" Value="[APPLICATIONFOLDER]${CLI.join("\\")}"` +
  ` Action="set" Part="last" System="no" Permanent="no"/>`;

/// `said` as a pattern that matches only itself.
function plain(said) {
  return said.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/// Either separator, because the project names each packed file with the build
/// host's own: `\` on Windows, and `/` on the wine the other two platforms would
/// reach for.
const SEP = "[\\\\/]";

/// The sidecar's own `File` element, and the indentation it was written at.
const SIDECAR = new RegExp(
  `^([ \\t]*)<File Name="${plain(EXE)}" ` +
    `Source="\\$\\(var\\.appDir\\)${SEP}${CLI.map(plain).join(SEP)}${SEP}${plain(EXE)}"[^\\n]*/>`,
  "m",
);

/// What this changes, in the order it changes it. Each is one thing the generated
/// project says and one thing it should have said.
const PATCHES = [
  {
    what: "the ApplicationFolderName property, which is what names the install directory",
    find: /(<Property Id="ApplicationFolderName" Value=")[^"]*(")/,
    put: `$1${DIRECTORY}$2`,
  },
  {
    what: "the APPLICATIONFOLDER directory, which is the install directory",
    find: /(<Directory Id="APPLICATIONFOLDER" Name=")[^"]*(")/,
    put: `$1${DIRECTORY}$2`,
  },
  {
    what: "the sidecar's own File element, which the PATH entry goes beside",
    find: SIDECAR,
    put: (element, indent) => `${element}\n${indent}${ENTRY}`,
  },
];

/// Apply one patch, or stop the pack saying which and why it matters.
function patch(project, { what, find, put }) {
  const found = project.match(new RegExp(find.source, `${find.flags}g`));
  const times = found === null ? 0 : found.length;

  if (times !== 1) {
    throw new Error(
      [
        times === 0
          ? `The generated WiX project is missing ${what}.`
          : `The generated WiX project has ${times} of ${what}, and this wants one.`,
        "",
        "electron-builder writes that project from a template of its own and this is",
        "string surgery on what it wrote, so a template that changed shape leaves nothing",
        "to change. Stopping here rather than packing an msi that installs under the",
        "wrong name or with no PATH entry in it — neither of which the install would say.",
        "",
        "What to read: desktop/scripts/msi.mjs, and templates/msi/template.xml inside",
        "app-builder-lib.",
      ].join("\n"),
    );
  }

  return project.replace(find, put);
}

/// The hook electron-builder calls, with the path of the project it just wrote.
export default async function msiProjectCreated(project) {
  const written = await readFile(project, "utf8");
  await writeFile(project, PATCHES.reduce(patch, written));
}
