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
//! `.icns` a Mac bundle carries and the `.ico` a Windows one does are staged
//! beside it, out of the same directory and for the same reason.
//!
//! Plain JavaScript and outside `src/`, for the reason `start.mjs` is: it is
//! what packs the app rather than part of it.
//!
//! **And on a Mac, which signature the app gets.** A Developer ID when
//! electron-builder's own `CSC_LINK` and `CSC_KEY_PASSWORD` hand it one, and
//! ad-hoc when they do not; and notarized when its `APPLE_API_KEY` trio hands
//! it an App Store Connect key as well — which is said here rather than in
//! `electron-builder.yml` because a configuration file cannot read whether a
//! variable is set. Either way the log says which, and afterwards by whose
//! name, out of the bundle that was packed (ADR-0027, *The release*).
//!
//! **And nothing is published.** The configuration names where an update is read
//! from, which is what has electron-builder write the update metadata; the
//! Release that metadata is uploaded to is the release workflow's to make.
//!
//! `pnpm run pack` rather than `pnpm pack`, which is pnpm's own command for
//! something else entirely. Everything after `--` is handed to
//! electron-builder, so a developer who wants one target of several can say so.

import { spawn, spawnSync } from "node:child_process";
import { chmodSync, copyFileSync, mkdirSync, rmSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { homedir } from "node:os";
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
  copyFileSync(join(ROOT, "packaging", `${APP_ID}.ico`), join(STAGED, "icon.ico"));
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
      "  cargo build --release -p verkstead-cli --target aarch64-apple-darwin",
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
    "In a checkout that is `cargo build --release -p verkstead-cli`, which is " +
      "the headless binary a Release ships.",
  ];
}

/// electron-builder itself — its own entry script, rather than the launcher the
/// install wrote for it in `node_modules/.bin`.
///
/// **Because Windows' launcher is a `.cmd` and Node will not spawn one.** Since
/// CVE-2024-27980 a batch file has to be run through a shell to be run at all,
/// and `spawn` refuses it outright with `EINVAL` — so the platform this stage is
/// about was the one platform `pnpm run pack` could not run on. Handing the
/// script to this Node is one spelling for all three, and it keeps the
/// developer's arguments out of a shell that would have re-quoted them.
function builder() {
  return createRequire(import.meta.url).resolve("electron-builder/cli.js");
}

/// Where electron-builder leaves the universal app before it images it, which is
/// what the signature is read back off. `directories.output` and the arch, in
/// electron-builder's own spelling.
const PACKED_APP = join(ROOT, "target", "electron", "out", "mac-universal", "Verkstead.app");

/// Whether this pack has a Developer ID to sign with: electron-builder's own
/// variable for the certificate, which it imports for the length of the pack.
/// The password beside it is the certificate's business, and a wrong or a
/// missing one is electron-builder's refusal to read it rather than this
/// script's guess.
function developerId() {
  return given("CSC_LINK");
}

/// The App Store Connect key electron-builder notarizes with, by its own three
/// variables: `APPLE_API_KEY` the path to the `.p8`, and the key's id and its
/// issuer's beside it.
const NOTARY = ["APPLE_API_KEY", "APPLE_API_KEY_ID", "APPLE_API_ISSUER"];

/// Whether a variable holds anything, which an empty repository secret does not.
function given(variable) {
  const value = process.env[variable];
  return value !== undefined && value.trim() !== "";
}

/// The file a `CSC_LINK` path names, resolved as electron-builder resolves it:
/// `~/` from the home directory, `file://` dropped, and anything relative from
/// this project, which is where the pack runs it.
function certificateFile(link) {
  if (link.startsWith("~/")) {
    return join(homedir(), link.slice(2));
  }
  return resolve(DESKTOP, link.startsWith("file://") ? link.slice("file://".length) : link);
}

/// What electron-builder is told on top of the configuration about signing and
/// notarizing, and the lines saying so.
///
/// Nothing, given a certificate and a key: the configuration is the Developer
/// ID one, and electron-builder notarizes and staples whenever the key is
/// there. Given a certificate alone, a Developer ID app that is not notarized,
/// said as `notarize: false` so that electron-builder logs the skip as one.
/// Without a certificate, the ad-hoc signature every release before ADR-0027
/// shipped, with the hardened runtime off beside it — see the `mac:` section of
/// `electron-builder.yml` for why the two go together — and nothing to
/// notarize, since Apple's notary takes a Developer ID's signature or nothing.
///
/// **Half a key is refused here** rather than handed on: electron-builder would
/// refuse it too, but only after an hour's universal build, and without saying
/// which of the three it was missing.
function signing() {
  if (!MAC) {
    return [];
  }

  if (!developerId()) {
    process.stdout.write(
      "Signing ad-hoc, because CSC_LINK names no Developer ID certificate, " +
        "and so not notarizing\n",
    );
    return [
      "--config.mac.identity=-",
      "--config.mac.hardenedRuntime=false",
      "--config.mac.notarize=false",
    ];
  }

  // **Never a certificate electron-builder would read as a file name.** It
  // takes `CSC_LINK` as base64 only when the value is over 2048 characters
  // or ends in `=`, and otherwise as a path — and a short `.p12`'s base64 is
  // neither, so electron-builder `stat`s it and prints the certificate whole
  // in its error. Refused here without the value: a file is what the release
  // hands over, and a path naming none is a certificate or a mistake, either
  // of which is better unprinted.
  const link = process.env.CSC_LINK.trim();
  const electronBuilderReadsAsBase64 =
    /^data:.*;base64,/.test(link) || link.length > 2048 || link.endsWith("=");
  if (
    !link.startsWith("https://") &&
    !electronBuilderReadsAsBase64 &&
    !statSync(certificateFile(link), { throwIfNoEntry: false })?.isFile()
  ) {
    die(
      "CSC_LINK names no file, and electron-builder would not read it as base64:",
      "",
      "write the .p12 to a file and name that, as the release leg does.",
    );
  }

  process.stdout.write("Signing with the Developer ID certificate in CSC_LINK\n");

  const missing = NOTARY.filter((variable) => !given(variable));
  if (missing.length === NOTARY.length) {
    process.stdout.write(`Not notarizing, because ${NOTARY.join(", ")} are not set\n`);
    return ["--config.mac.notarize=false"];
  }
  if (missing.length > 0) {
    die(
      `${missing.join(" and ")} ${missing.length > 1 ? "are" : "is"} not set, ` +
        "so there is only part of an App Store Connect key to notarize with:",
      "",
      `notarizing takes all three of ${NOTARY.join(", ")}, and packing takes none of them.`,
    );
  }
  if (!statSync(process.env.APPLE_API_KEY, { throwIfNoEntry: false })?.isFile()) {
    die(
      `APPLE_API_KEY is ${process.env.APPLE_API_KEY}, and there is no file there:`,
      "",
      "it is the path to the key's .p8 rather than the key itself.",
    );
  }

  process.stdout.write(
    `Notarizing with the App Store Connect key ${process.env.APPLE_API_KEY_ID}, ` +
      "and stapling the ticket to the app\n",
  );
  return [];
}

/// Say whose signature the packed app carries, read off the bundle rather than
/// off which branch was taken: `codesign -dv` prints an `Authority=` line per
/// certificate in the chain for a real identity and `Signature=adhoc` for the
/// other, and the runtime among its flags.
///
/// A pack told to build one architecture alone leaves no universal app, and
/// that is a developer's choice rather than a failure, so it is said and left.
function signed() {
  if (!statSync(PACKED_APP, { throwIfNoEntry: false })?.isDirectory()) {
    process.stdout.write(`There is no app at ${PACKED_APP}, so its signature goes unread\n`);
    return;
  }

  const run = spawnSync("codesign", ["-dv", "--verbose=2", PACKED_APP], { encoding: "utf8" });
  // codesign writes what it displays to standard error.
  const said = `${run.stdout ?? ""}${run.stderr ?? ""}`;

  if (run.error !== undefined || run.status !== 0) {
    die(`codesign could not read the signature on ${PACKED_APP}:`, "", said.trimEnd());
  }

  const authority = said.match(/^Authority=(.*)$/m)?.[1];

  // **A certificate that signed nothing is a failed pack**, not an ad-hoc one:
  // electron-builder, finding no identity in what it imported — a `.p12`
  // exported without its private key does exactly this — logs a skip and
  // carries on, and the app comes out signed ad-hoc by the framework's own
  // signatures. A `.p12` holding the key without its certificate does the
  // same — what exporting the key's row alone writes. Said here, where the
  // developer asked for a Developer ID, rather than discovered by whatever
  // reads the bundle next.
  if (developerId() && authority === undefined) {
    die(
      "CSC_LINK was given a certificate, and the app came out signed ad-hoc:",
      "",
      "electron-builder found no Developer ID Application identity in it. An identity",
      "is the certificate and its private key together, and a .p12 holding only one",
      "of them has none — export the certificate's row under My Certificates, with",
      "its key beneath it.",
    );
  }
  const runtime = /flags=\S*runtime/.test(said) ? ", with the hardened runtime" : "";
  process.stdout.write(
    authority !== undefined
      ? `The app is signed by ${authority}${runtime}\n`
      : `The app is signed ad-hoc${runtime}\n`,
  );

  // And whether a notarization ticket is stapled inside it, which is the other
  // half of what Gatekeeper reads off a download and the half `codesign` does
  // not show. `stapler` exits non-zero for a bundle with none, which an ad-hoc
  // or an unnotarized pack is meant to be.
  const stapled = spawnSync("xcrun", ["stapler", "validate", PACKED_APP], { encoding: "utf8" });
  process.stdout.write(
    stapled.error === undefined && stapled.status === 0
      ? "The app carries a stapled notarization ticket\n"
      : "The app carries no notarization ticket\n",
  );
}

const binaries = cli();

process.stdout.write(`The sidecar is ${binaries.join(" and ")}\n`);
stage(binaries);

const signs = signing();

/// Whether this pack notarizes, which is `signing` handing electron-builder
/// nothing on top of the configuration on a Mac — see there.
const notarizing = MAC && signs.length === 0;

/// How many times a pack whose staple alone failed is packed again.
const STAPLE_ATTEMPTS = 3;

/// What electron-builder logs when `stapler` could not fetch the ticket it was
/// to staple, from `@electron/notarize`.
const STAPLE_FAILED = "Failed to staple your application";

/// **A staple that fails is packed again**, because the notary has already
/// accepted the app by then and what failed is `stapler` fetching the ticket
/// from Apple's CloudKit — a connection Apple's end drops now and then ("The
/// network connection was lost", error 68). electron-builder staples inside
/// its notarizing and has no hook between the two, so the pack is what is run
/// again; the notary accepts the same app a second time. Any other failure is
/// the pack's own and is not retried. Piped only when notarizing, so that
/// every other pack keeps electron-builder's terminal as it was.
function pack(attempt) {
  // No platform flag: electron-builder packs for the machine it is running on,
  // which is what each of the three artifacts has always been built by — and what
  // keeps this one command rather than one per platform.
  //
  // **And never a publish**, said on every pack rather than only a release
  // leg's: `electron-builder.yml` names a `github` provider so that the update
  // metadata is written, and electron-builder left to itself publishes to it
  // whenever it finds itself on CI at a tag. The Release is
  // `.github/workflows/release.yml`'s to make, metadata and all.
  const packing = spawn(
    process.execPath,
    [builder(), "--publish", "never", ...signs, ...args],
    { cwd: DESKTOP, stdio: notarizing ? ["inherit", "pipe", "pipe"] : "inherit" },
  );

  let stapleFailed = false;
  if (notarizing) {
    for (const [from, to] of [
      [packing.stdout, process.stdout],
      [packing.stderr, process.stderr],
    ]) {
      // With the end of the chunk before, so that a line split across two
      // is still read.
      let tail = "";
      from.on("data", (chunk) => {
        to.write(chunk);
        const read = tail + chunk.toString();
        stapleFailed ||= read.includes(STAPLE_FAILED);
        tail = read.slice(-STAPLE_FAILED.length);
      });
    }
  }

  packing.on("close", (code, signal) => {
    const status = signal !== null ? 1 : (code ?? 0);
    if (status !== 0 && stapleFailed && attempt < STAPLE_ATTEMPTS) {
      process.stdout.write(
        `The notarization ticket could not be stapled, so packing again ` +
          `(attempt ${attempt + 1} of ${STAPLE_ATTEMPTS})\n`,
      );
      pack(attempt + 1);
      return;
    }
    if (status === 0 && MAC) {
      signed();
    }
    // Ended by running out of work rather than by `process.exit`, which drops
    // whatever of the piped output is still queued — and on a Mac a pipe is
    // written asynchronously, so that is the tail of a failing pack's log.
    process.exitCode = status;
  });
}

pack(1);
