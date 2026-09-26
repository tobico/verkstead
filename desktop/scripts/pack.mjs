//! `pnpm run pack` — the app packed for this machine's platform, carrying the
//! CLI you name.
//!
//! **The CLI is the point of this script.** electron-builder packs what
//! `electron-builder.yml` names, and the one thing that file cannot name is
//! which `verkstead` the app is to carry: in a Release it is the static musl
//! binary the same run's CLI leg built (ADR-0020), and in a checkout it is
//! whatever the developer has. So it is said here — `VERKSTEAD_CLI`, the same
//! variable the app itself takes an override from (see `src/cli.ts`), or a path
//! as the first argument — and staged where the configuration reads it from.
//!
//! **And the icons, which are staged rather than read where they lie.**
//! electron-builder reads an icon directory as a flat set of `<size>x<size>.png`
//! files; `packaging/icons/hicolor` is the tree a desktop *installs*, which
//! names every file for the app id and puts each in a directory of its own. The
//! pixels are the same either way — this is a rename, and one that keeps
//! `tools/generate-packaging.sh` the only thing that writes that tree.
//!
//! Plain JavaScript and outside `src/`, for the reason `start.mjs` is: it is
//! what packs the app rather than part of it.
//!
//! `pnpm run pack` rather than `pnpm pack`, which is pnpm's own command for
//! something else entirely. Everything after `--` is handed to
//! electron-builder, so a developer who wants one target of several can say so.

import { spawn } from "node:child_process";
import { copyFileSync, mkdirSync, rmSync, statSync } from "node:fs";
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

function die(...said) {
  process.stderr.write(`${said.join("\n")}\n`);
  process.exit(1);
}

/// What was said on the command line: a path, and then whatever is
/// electron-builder's.
///
/// The path is optional and the flags are optional, so the two are told apart by
/// the one thing that distinguishes them — a flag begins with a dash and a path
/// does not.
const args = process.argv.slice(2);
const said = args[0] !== undefined && !args[0].startsWith("-") ? args.shift() : undefined;

/// The binary this pack is to carry, said by the developer or by the release
/// leg — and never guessed at. A pack that fell back to the debug build would be
/// an artifact whose sidecar is nobody's decision.
function cli() {
  const path = said ?? process.env.VERKSTEAD_CLI;

  if (path === undefined || path === "") {
    die(
      "Which verkstead is this to carry? Say it as the first argument, or in " +
        "VERKSTEAD_CLI:",
      "",
      "  pnpm run pack ../target/release/verkstead",
      "",
      "In a checkout that is `cargo build --release -p verkstead-cli " +
        "--no-default-features`, which is the headless binary a Release ships.",
    );
  }

  const resolved = resolve(path);

  if (!statSync(resolved, { throwIfNoEntry: false })?.isFile()) {
    die(`There is no file at ${resolved}, so there is nothing to pack.`);
  }

  return resolved;
}

/// What the binary is called inside the pack — `src/cli.ts`'s own answer, which
/// is the platform's rather than the file's. The pack is for the machine it runs
/// on, as each of the three artifacts always has been.
function named() {
  return process.platform === "win32" ? "verkstead.exe" : "verkstead";
}

/// Put the CLI and the icons where `electron-builder.yml` reads them from.
///
/// From nothing every time: a staging directory that kept last run's binary is
/// one that can pack a CLI nobody asked for.
function stage(binary) {
  rmSync(STAGED, { force: true, recursive: true });

  const packed = join(STAGED, "cli", named());
  mkdirSync(dirname(packed), { recursive: true });
  copyFileSync(binary, packed);

  const icons = join(STAGED, "icons");
  mkdirSync(icons, { recursive: true });
  for (const size of SIZES) {
    copyFileSync(
      join(ROOT, "packaging", "icons", "hicolor", `${size}x${size}`, "apps", `${APP_ID}.png`),
      join(icons, `${size}x${size}.png`),
    );
  }
}

/// electron-builder itself, which the install has put a command for in
/// `node_modules/.bin`.
function builder() {
  const name = process.platform === "win32" ? "electron-builder.cmd" : "electron-builder";
  return join(DESKTOP, "node_modules", ".bin", name);
}

const binary = cli();

process.stdout.write(`The sidecar is ${binary}\n`);
stage(binary);

// No platform flag: electron-builder packs for the machine it is running on,
// which is what each of the three artifacts has always been built by — and what
// keeps this one command rather than one per platform.
const packing = spawn(builder(), args, {
  cwd: DESKTOP,
  stdio: "inherit",
});

packing.on("exit", (code, signal) => process.exit(signal !== null ? 1 : (code ?? 0)));
