//! `node scripts/updates.mjs` — what a release leg reads back about the update
//! electron-updater will be handed, out of the files that are uploaded.
//!
//! Two questions, one per subcommand, and each exits non-zero with what was
//! wrong:
//!
//! - **`metadata <latest*.yml> <asset>...`** — the metadata electron-builder
//!   wrote beside a platform's artifacts names those artifacts, by the names the
//!   Release will hold them under, with the size and the sha512 each file
//!   really has. The first asset is the one the metadata's `path` must name:
//!   the file an update installs. Every `files` entry must be one of the
//!   assets, and every asset must be among them. This is the whole of what an
//!   update rests on while the metadata is unsigned (ADR-0027, *The release*):
//!   a name the Release does not hold is an update that 404s, and a stale hash
//!   is one electron-updater downloads and throws away.
//! - **`packed <resources> <owner/repo>`** — the packed app's resources
//!   directory carries the `app-update.yml` electron-updater reads at runtime,
//!   naming the `github` provider and this repository, and `app.asar` carries
//!   electron-updater itself.
//!
//! Plain JavaScript and outside `src/`, for the reason `pack.mjs` is: it reads
//! what was packed rather than being part of it. It runs on all three runners,
//! so it is Node rather than a shell's `sha512sum`, which a Mac spells
//! differently.

import { createHash } from "node:crypto";
import { readFileSync, statSync } from "node:fs";
import { createRequire } from "node:module";
import { basename, join } from "node:path";

/// The YAML reader electron-updater itself reads these files with — reached
/// through electron-updater, since it is that package's dependency rather than
/// this project's, and pnpm lays out only a project's own.
const yaml = createRequire(createRequire(import.meta.url).resolve("electron-updater"))(
  "js-yaml",
);

function die(...said) {
  process.stderr.write(`${said.join("\n")}\n`);
  process.exit(1);
}

function read(path) {
  if (!statSync(path, { throwIfNoEntry: false })?.isFile()) {
    die(`There is no ${path}.`);
  }
  return readFileSync(path);
}

/// A file's sha512, in the base64 electron-builder writes it in.
function sha512(path) {
  return createHash("sha512").update(read(path)).digest("base64");
}

function metadata(file, assets) {
  if (file === undefined || assets.length === 0) {
    die("Usage: node scripts/updates.mjs metadata <latest*.yml> <asset>...");
  }

  const info = yaml.load(read(file).toString("utf8"));
  const named = new Map(assets.map((asset) => [basename(asset), asset]));
  const installed = basename(assets[0]);
  const wrong = [];

  if (info.path !== installed) {
    wrong.push(`its path is ${info.path}, and the update is ${installed}`);
  }
  if (info.sha512 !== sha512(assets[0])) {
    wrong.push(`its sha512 is not ${installed}'s`);
  }

  const listed = new Set();
  for (const entry of info.files ?? []) {
    listed.add(entry.url);
    const asset = named.get(entry.url);
    if (asset === undefined) {
      wrong.push(`it names ${entry.url}, which is not among the assets`);
      continue;
    }
    if (entry.sha512 !== sha512(asset)) {
      wrong.push(`its sha512 for ${entry.url} is not that file's`);
    }
    if (entry.size !== statSync(asset).size) {
      wrong.push(`its size for ${entry.url} is ${entry.size}, not ${statSync(asset).size}`);
    }
  }
  for (const name of named.keys()) {
    if (!listed.has(name)) {
      wrong.push(`it does not list ${name}`);
    }
  }

  if (wrong.length > 0) {
    die(`${file} does not describe what is uploaded:`, "", ...wrong.map((line) => `- ${line}`));
  }

  process.stdout.write(
    `${basename(file)} names ${[...named.keys()].join(" and ")} at version ${info.version}, ` +
      "with each file's size and sha512\n",
  );
}

/// The paths at the top of an asar archive's `node_modules`, read out of its
/// header: a pickle whose length is at byte 12 and whose JSON follows at 16.
function modules(asar) {
  const archive = read(asar);
  const header = JSON.parse(archive.subarray(16, 16 + archive.readUInt32LE(12)).toString("utf8"));
  return Object.keys(header.files?.node_modules?.files ?? {});
}

function packed(resources, repository) {
  if (resources === undefined || repository === undefined) {
    die("Usage: node scripts/updates.mjs packed <resources> <owner/repo>");
  }

  const config = yaml.load(read(join(resources, "app-update.yml")).toString("utf8"));
  const [owner, repo] = repository.split("/");
  const wrong = [];

  if (config.provider !== "github") {
    wrong.push(`app-update.yml names the ${config.provider} provider, not github`);
  }
  if (config.owner?.toLowerCase() !== owner.toLowerCase() ||
    config.repo?.toLowerCase() !== repo?.toLowerCase()) {
    wrong.push(`app-update.yml names ${config.owner}/${config.repo}, not ${repository}`);
  }
  if (!modules(join(resources, "app.asar")).includes("electron-updater")) {
    wrong.push("app.asar carries no electron-updater");
  }

  if (wrong.length > 0) {
    die(`The app in ${resources} cannot update itself:`, "", ...wrong.map((line) => `- ${line}`));
  }

  process.stdout.write(
    `The app reads its updates from ${repository}'s Releases, and carries electron-updater\n`,
  );
}

const [command, ...args] = process.argv.slice(2);

if (command === "metadata") {
  metadata(args[0], args.slice(1));
} else if (command === "packed") {
  packed(args[0], args[1]);
} else {
  die("Usage: node scripts/updates.mjs metadata|packed ...");
}
