//! `pnpm run pack` — the app packed for this machine's platform, carrying the
//! CLI you name.
//!
//! **The CLI is the point of this script.** electron-builder packs what
//! `electron-builder.yml` names, and the one thing that file cannot name is
//! which `verkstead` the app is to carry: in a Release it is the binary the same
//! run's CLI leg built (ADR-0020), and in a checkout it is whatever the developer
//! has. So it is said here — `VERKSTEAD_CLI`, the same variable the app itself
//! takes an override from (see `src/cli.ts`), or a path as the first argument —
//! and staged where the configuration reads it from.
//!
//! **A Mac is given two of them, and this is what joins them.** The download
//! there is one universal app for both Apple machines, so the sidecar inside it
//! has to be universal as well: `lipo` over both Apple builds, written here
//! rather than in the release leg so that a developer packs the way the leg does
//! (ADR-0020). Which is also what a universal pack *requires* —
//! @electron/universal refuses a resource that differs between the two halves it
//! merges, and a file that is both architectures already is the same file in
//! both.
//!
//! **And the artwork, which is staged rather than read where it lies.**
//! electron-builder reads an icon directory as a flat set of `<size>x<size>.png`
//! files; `packaging/icons/hicolor` is the tree a desktop *installs*, which
//! names every file for the app id and puts each in a directory of its own. The
//! pixels are the same either way — this is a rename, and one that keeps
//! `tools/generate-packaging.sh` the only thing that writes that tree. The
//! `.icns` a Mac bundle carries is staged beside it, out of the same directory
//! and for the same reason.
//!
//! Plain JavaScript and outside `src/`, for the reason `start.mjs` is: it is
//! what packs the app rather than part of it.
//!
//! `pnpm run pack` rather than `pnpm pack`, which is pnpm's own command for
//! something else entirely. Everything after `--` is handed to
//! electron-builder, so a developer who wants one target of several can say so.

import { spawn, spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdirSync, rmSync, statSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

/// This project, which is one directory up from this script.
const DESKTOP = resolve(import.meta.dirname, "..");

/// And the repository, which is one further: what is staged below comes out of
/// the working tree rather than out of this project.
const ROOT = resolve(DESKTOP, "..");

/// Where the configuration reads the staged files from. `target/` because that
/// is the directory cargo already writes to, so one `.gitignore` line covers it
/// — see `directories` in `electron-builder.yml`.
const STAGED = join(ROOT, "target", "electron", "staged");

/// The sizes `tools/generate-packaging.sh` writes, which are the sizes a panel,
/// a menu and the image's own `.DirIcon` are drawn from.
const SIZES = [16, 24, 32, 48, 64, 128, 256, 512];

/// The app id, which is what the committed artwork is named for.
const APP_ID = "net.tobico.Verkstead";

/// Whether this pack is a Mac's, which is the one platform that wants two
/// binaries rather than one.
const MAC = process.platform === "darwin";

/// How many binaries this pack is given: both Apple builds on a Mac, and the one
/// binary it has always been everywhere else.
const WANTED = MAC ? 2 : 1;

/// The architectures the joined sidecar has to hold, by the names `lipo` knows
/// them by — which is rustc's name for one of them and not for the other.
///
/// Read off the file that was written rather than off which argument was which:
/// `lipo` takes each half's architecture from the half itself, so the two can be
/// named in either order, and what matters is that both came out the other end.
const ARCHS = ["arm64", "x86_64"];

function die(...said) {
  process.stderr.write(`${said.join("\n")}\n`);
  process.exit(1);
}

/// What was said on the command line: the paths, and then whatever is
/// electron-builder's.
///
/// The paths are optional and the flags are optional, so the two are told apart
/// by the one thing that distinguishes them — a flag begins with a dash and a
/// path does not.
const args = process.argv.slice(2);
const said = [];
while (args[0] !== undefined && !args[0].startsWith("-")) {
  said.push(args.shift());
}

/// The binaries this pack is to carry, said by the developer or by the release
/// leg — and never guessed at. A pack that fell back to the debug build would be
/// an artifact whose sidecar is nobody's decision.
///
/// `VERKSTEAD_CLI` is the one-path spelling and stays exactly what it was: a
/// whole sidecar on Linux and on Windows, and half of one on a Mac, where the
/// two are said as the two arguments.
function cli() {
  const variable = process.env.VERKSTEAD_CLI;
  const paths =
    said.length > 0 ? said : variable !== undefined && variable !== "" ? [variable] : [];

  if (paths.length !== WANTED) {
    die(...asking(paths.length));
  }

  return paths.map((path) => {
    const resolved = resolve(path);

    if (!statSync(resolved, { throwIfNoEntry: false })?.isFile()) {
      die(`There is no file at ${resolved}, so there is nothing to pack.`);
    }

    return resolved;
  });
}

/// What the binary is called inside the pack — `src/cli.ts`'s own answer, which
/// is the platform's rather than the file's. The pack is for the machine it runs
/// on, as each of the three artifacts always has been.
function named() {
  return process.platform === "win32" ? "verkstead.exe" : "verkstead";
}

/// Put the CLI and the artwork where `electron-builder.yml` reads them from.
///
/// From nothing every time: a staging directory that kept last run's binary is
/// one that can pack a CLI nobody asked for.
///
/// Every platform's artwork is staged wherever this runs, rather than the
/// platform's own: staging is a copy, and the configuration a Mac reads names the
/// `.icns` where the one a Linux box reads names the icon directory — so a pack
/// that staged by platform would be the same three copies behind a condition.
function stage(binaries) {
  rmSync(STAGED, { force: true, recursive: true });

  const packed = join(STAGED, "cli", named());
  mkdirSync(dirname(packed), { recursive: true });
  if (binaries.length > 1) {
    universal(binaries, packed);
  } else {
    copyFileSync(binaries[0], packed);
  }

  const icons = join(STAGED, "icons");
  mkdirSync(icons, { recursive: true });
  for (const size of SIZES) {
    copyFileSync(
      join(ROOT, "packaging", "icons", "hicolor", `${size}x${size}`, "apps", `${APP_ID}.png`),
      join(icons, `${size}x${size}.png`),
    );
  }

  copyFileSync(join(ROOT, "packaging", `${APP_ID}.icns`), join(STAGED, "icon.icns"));
}

/// Join both Apple builds into the one file at `packed`, and read back that it
/// really is both.
///
/// `lipo` copies each half whole and writes a header in front of them saying
/// which is which, and the loader takes the half the Mac it is on runs. Checked
/// rather than trusted, as the Rust bundle this replaced checked its own: a
/// `lipo` given one input writes a perfectly good single-architecture file, and
/// the Mac that could not run it is the one nobody testing this has.
function universal(halves, packed) {
  lipo(["-create", "-output", packed, ...halves]);

  // Executable, said rather than left to what `lipo` wrote: the app spawns this
  // file as its sidecar, and a copy of the mode is what the one-binary arm above
  // gets from `copyFileSync` for free.
  chmodSync(packed, 0o755);

  const archs = lipo(["-archs", packed]).split(/\s+/).filter(Boolean);
  for (const arch of ARCHS) {
    if (!archs.includes(arch)) {
      die(
        `The joined sidecar holds ${archs.join(" and ")}, so it has no ${arch} in it:`,
        "",
        "a universal app carries a universal sidecar, and what was named here is not",
        "both Apple builds.",
      );
    }
  }

  process.stdout.write(`The sidecar holds ${archs.join(" and ")}\n`);
}

/// `lipo` with these arguments, and what it said. It is the operating system's
/// own tool, so a Mac has one and nothing else runs this.
function lipo(said) {
  const run = spawnSync("lipo", said, { encoding: "utf8" });

  if (run.error !== undefined || run.status !== 0) {
    die(
      `lipo ${said.join(" ")} did not work:`,
      "",
      (run.stderr ?? String(run.error)).trimEnd(),
    );
  }

  return run.stdout;
}

/// What to say to somebody who named the wrong number of binaries, which is a
/// different sentence on a Mac.
function asking(given) {
  if (MAC) {
    return [
      given === 0
        ? "Which verkstead builds is this to carry?"
        : `${given} binaries were named, and a Mac's pack takes two.`,
      "",
      "The app is universal, so the sidecar inside it is too: say both Apple builds,",
      "in either order.",
      "",
      "  pnpm run pack ../target/aarch64-apple-darwin/release/verkstead \\",
      "    ../target/x86_64-apple-darwin/release/verkstead",
      "",
      "In a checkout each is the headless binary a Release ships, built for one",
      "Apple target:",
      "",
      "  cargo build --release -p verkstead-cli --no-default-features \\",
      "    --target aarch64-apple-darwin",
    ];
  }

  return [
    given === 0
      ? "Which verkstead is this to carry? Say it as the first argument, or in " +
        "VERKSTEAD_CLI:"
      : `${given} binaries were named, and this pack takes one:`,
    "",
    "  pnpm run pack ../target/release/verkstead",
    "",
    "In a checkout that is `cargo build --release -p verkstead-cli " +
      "--no-default-features`, which is the headless binary a Release ships.",
  ];
}

/// electron-builder itself, which the install has put a command for in
/// `node_modules/.bin`.
function builder() {
  const name = process.platform === "win32" ? "electron-builder.cmd" : "electron-builder";
  return join(DESKTOP, "node_modules", ".bin", name);
}

const binaries = cli();

process.stdout.write(`The sidecar is ${binaries.join(" and ")}\n`);
stage(binaries);

// No platform flag: electron-builder packs for the machine it is running on,
// which is what each of the three artifacts has always been built by — and what
// keeps this one command rather than one per platform.
const packing = spawn(builder(), args, {
  cwd: DESKTOP,
  stdio: "inherit",
});

packing.on("exit", (code, signal) => process.exit(signal !== null ? 1 : (code ?? 0)));
