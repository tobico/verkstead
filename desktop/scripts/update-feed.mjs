//! `node scripts/update-feed.mjs pack <sidecar> <dir>`, then
//! `node scripts/update-feed.mjs run <dir>` — this device's Update, end to end,
//! with the real electron-updater against a local feed (ADR-0027, *A device
//! updates itself*).
//!
//! **Two steps, because the second wants a machine to itself.** `pack` packs
//! the app twice into `<dir>`, which wants the network electron-builder fetches
//! from; `run` takes port 8422 and a screen, and runs equally well in a network
//! namespace of its own with no network at all.
//!
//! **What it proves is the exchange on a packed app**, which vitest's fake
//! cannot: the app is packed twice, at [`LOW`] and at [`HIGH`], the higher
//! one's `latest-linux.yml` and AppImage are served from a `generic` feed on
//! loopback, and the lower one is started pointed at it with
//! `VERKSTEAD_UPDATE_FEED` — the override only a test sets. Then Update is
//! pressed through the API, the way the row presses it, and the run is read
//! back: the row reads *updating…*, the server stops answering, a server
//! answers again with no Update going, the image on disk is the higher one,
//! and the app's log says the higher version started, waited out the one it
//! replaced, and neither handed over to it nor met a foreign listener.
//!
//! **And first, an Update that fails** (ADR-0027: no rollback — the trouble
//! line, and a higher release). The lower one is started with a version
//! remembered as handed over to that it isn't, which is the install that
//! failed quietly: the row carries the trouble line from the start. Then it is
//! pointed at the feed's `corrupt/`, which serves the same metadata over an
//! image with one byte changed, and Update is pressed: the row leaves
//! *updating…* for a trouble line naming the sha512 mismatch, the server never
//! stops answering, and the image on disk is the one it started from. That
//! app is stopped, and the good run starts afresh.
//!
//! **The version that moves is the app's.** Both packs carry the one sidecar
//! this is handed, so the server answers the same release before and after;
//! what says the relaunch is the new release is the app's own start line, and
//! the file. A real release moves both, being one version.
//!
//! **It wants a screen**, as the release leg's runs do: `DISPLAY` set, and a
//! session bus where the tray is to be raised. And port 8422 free, which is
//! the one address the app serves on. Run by the Linux desktop leg of
//! `.github/workflows/release.yml`; locally, on a Linux machine with FUSE, run
//! it the same way.
//!
//! `VERKSTEAD_FEED_ZERO_ABI=1` is for a machine whose kernel refuses the
//! AppImage magic in the ELF header (see the memo on running a packed
//! AppImage): it zeroes those three bytes in both images and serves metadata
//! for the zeroed file, and has the runtime extract rather than mount. Nothing
//! a runner needs.

import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  chmodSync,
  copyFileSync,
  createReadStream,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  readSync,
  statSync,
  writeFileSync,
  writeSync,
  closeSync,
} from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const DESKTOP = resolve(import.meta.dirname, "..");

/// The two releases: the one installed, and the one the feed offers.
const LOW = "0.0.1";
const HIGH = "0.0.2";

/// What both packs name their image — `appImage.artifactName`.
const IMAGE = "Verkstead-x86_64.AppImage";

/// The app's one address, and the key the server is started with.
const ORIGIN = "http://127.0.0.1:8422";
const KEY = "the-key-the-feed-run-states";

/// How long each wait is given: a first start decompresses a whole Electron,
/// and the download is a hundred megabytes over loopback. In milliseconds,
/// and `VERKSTEAD_FEED_PATIENCE` says otherwise for a run that wants an answer
/// sooner.
const PATIENCE = Number(process.env.VERKSTEAD_FEED_PATIENCE ?? 120_000);

function die(...said) {
  process.stderr.write(`${said.join("\n")}\n`);
  process.exit(1);
}

const pause = (ms) => new Promise((done) => setTimeout(done, ms));

const USAGE = [
  "Usage: node scripts/update-feed.mjs pack <sidecar> <dir>",
  "       node scripts/update-feed.mjs run <dir>",
];

/// Pack the app at `version` into `out`, carrying `sidecar`.
function pack(sidecar, version, out) {
  process.stdout.write(`Packing Verkstead ${version}\n`);
  const packed = spawnSync(
    process.execPath,
    [
      join(DESKTOP, "scripts", "pack.mjs"),
      resolve(sidecar),
      `-c.extraMetadata.version=${version}`,
      `-c.directories.output=${out}`,
    ],
    { cwd: DESKTOP, stdio: "inherit" },
  );
  if (packed.status !== 0) {
    die(`Packing ${version} failed.`);
  }
  return join(out, IMAGE);
}

/// A file's sha512, in electron-builder's base64.
const sha512 = (path) => createHash("sha512").update(readFileSync(path)).digest("base64");

/// The three bytes of AppImage magic, zeroed — see the top of this file.
function zero(path) {
  const file = openSync(path, "r+");
  writeSync(file, Buffer.alloc(3), 0, 3, 8);
  closeSync(file);
}

const [step, ...rest] = process.argv.slice(2);

if (step === "pack") {
  const [sidecar, dir] = rest;
  if (sidecar === undefined || dir === undefined || !existsSync(sidecar)) {
    die(...USAGE);
  }
  const built = spawnSync("pnpm", ["build"], { cwd: DESKTOP, stdio: "inherit" });
  if (built.status !== 0) {
    die("The app would not build.");
  }
  pack(sidecar, LOW, join(resolve(dir), "low"));
  pack(sidecar, HIGH, join(resolve(dir), "high"));
  process.exit(0);
}

if (step !== "run" || rest[0] === undefined) {
  die(...USAGE);
}
if (!process.env.DISPLAY) {
  die("There is no DISPLAY, and the app opens a window.");
}

const packed = resolve(rest[0]);
const low = join(packed, "low", IMAGE);
const highOut = join(packed, "high");
const high = join(highOut, IMAGE);
for (const image of [low, high]) {
  if (!existsSync(image)) {
    die(`There is no ${image}. Pack it first.`, ...USAGE);
  }
}

const zeroing = process.env.VERKSTEAD_FEED_ZERO_ABI === "1";
const work = mkdtempSync(join(process.env.VERKSTEAD_FEED_WORK ?? tmpdir(), "vkf-"));
process.stdout.write(`Working in ${work}\n`);

// The feed: the higher pack's metadata and image, copied out of the pack so a
// run leaves the pack as it was, and served as a `generic` provider reads one —
// `latest-linux.yml` and the file it names. A single range is served, and the
// several-at-once request a differential download makes is answered whole,
// which electron-updater takes as its cue to download the whole file — at
// [`RATE`], so the row's reports are there to be read. Under `corrupt/`, the
// same metadata over an image with one byte changed, which no sha512 check
// passes.
const served = join(work, "feed");
mkdirSync(join(served, "corrupt"), { recursive: true });
const metadata = join(served, "latest-linux.yml");
copyFileSync(join(highOut, "latest-linux.yml"), metadata);
copyFileSync(high, join(served, IMAGE));
const offering = join(served, IMAGE);
if (zeroing) {
  const before = sha512(offering);
  zero(offering);
  const after = sha512(offering);
  writeFileSync(
    metadata,
    readFileSync(metadata, "utf8").replaceAll(before, after),
  );
  process.stdout.write("The feed's image has its ABI bytes zeroed, and its metadata says so\n");
}
const offered = sha512(offering);

copyFileSync(metadata, join(served, "corrupt", "latest-linux.yml"));
const corrupt = join(served, "corrupt", IMAGE);
copyFileSync(offering, corrupt);
{
  const file = openSync(corrupt, "r+");
  const at = Math.floor(statSync(corrupt).size / 2);
  const byte = Buffer.alloc(1);
  readSync(file, byte, 0, 1, at);
  byte[0] ^= 0xff;
  writeSync(file, byte, 0, 1, at);
  closeSync(file);
}

/// How fast the feed serves an image: a loopback download is otherwise done
/// before the row has been read once.
const RATE = 24 * 1024 * 1024;

/// Send `path` from `start` to `end` at [`RATE`].
async function trickle(path, response, start, end) {
  const every = 50;
  const stream = createReadStream(path, { start, end, highWaterMark: Math.floor((RATE * every) / 1000) });
  for await (const chunk of stream) {
    if (!response.write(chunk)) {
      await new Promise((drained) => response.once("drain", drained));
    }
    await pause(every);
  }
  response.end();
}

const asked = [];
const feed = createServer((request, response) => {
  asked.push(`${request.method} ${request.url}`);
  const name = decodeURIComponent(new URL(request.url, "http://feed").pathname).replace(/^\/+/, "");
  const path = join(served, name);
  if (name.replace(/^corrupt\//, "").includes("/") || !existsSync(path) || !statSync(path).isFile()) {
    response.statusCode = 404;
    response.end();
    return;
  }

  const size = statSync(path).size;
  const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? "");
  if (range === null) {
    response.writeHead(200, { "content-length": size, "accept-ranges": "bytes" });
    void trickle(path, response, 0, size - 1);
    return;
  }
  const start = Number(range[1]);
  const end = range[2] === "" ? size - 1 : Math.min(Number(range[2]), size - 1);
  response.writeHead(206, {
    "content-length": end - start + 1,
    "content-range": `bytes ${start}-${end}/${size}`,
  });
  void trickle(path, response, start, end);
});
await new Promise((up) => feed.listen(0, "127.0.0.1", up));
const FEED = `http://127.0.0.1:${feed.address().port}/`;
process.stdout.write(`The feed is ${FEED}\n`);

/// The run going: its log, its install, and the app it started.
let run;

/// Start the lower image as a fresh install under `name`, reading `feed`:
/// under its release name, which has electron-updater overwrite it in place
/// rather than write a second file, with a Data Directory, a state, a config
/// and a cache of its own. `remembered` is a version left as handed over to,
/// for the start to find it didn't land.
function launch(name, feedUrl, remembered) {
  const home = join(work, name);
  const installed = join(home, "app", IMAGE);
  mkdirSync(join(home, "app"), { recursive: true });
  copyFileSync(low, installed);
  chmodSync(installed, 0o755);
  if (zeroing) {
    zero(installed);
  }

  const data = join(home, "data");
  mkdirSync(data);
  writeFileSync(join(data, "workbench.key"), `${KEY}\n`);

  if (remembered !== undefined) {
    // Electron's userData on Linux: the config home and the package's name.
    const userData = join(home, "config", "verkstead-desktop");
    mkdirSync(userData, { recursive: true });
    writeFileSync(join(userData, "updating.json"), JSON.stringify({ version: remembered }));
  }

  // Without `VERKSTEAD_CLI`, which the step that packed the feed is handed
  // and which the app honours over its own bundle (`src/cli.ts`): left in, the
  // app starts the step's loose binary, which reads itself as a bare `Binary`
  // install and refuses the Update this run is here to press.
  const env = {
    ...process.env,
    VERKSTEAD_DATA_DIR: data,
    VERKSTEAD_UPDATE_FEED: feedUrl,
    XDG_STATE_HOME: join(home, "state"),
    XDG_CONFIG_HOME: join(home, "config"),
    XDG_CACHE_HOME: join(home, "cache"),
    ...(zeroing ? { APPIMAGE_EXTRACT_AND_RUN: "1" } : {}),
  };
  delete env.VERKSTEAD_CLI;

  const output = openSync(join(home, "app.out"), "w");
  run = {
    installed,
    started: sha512(installed),
    log: join(home, "state", "verkstead", "verkstead.log"),
    app: spawn(installed, [], { env, stdio: ["ignore", output, output] }),
  };
  process.stdout.write(`Started the ${name} run\n`);
}

const said = () => (run !== undefined && existsSync(run.log) ? readFileSync(run.log, "utf8") : "");

/// Every pid the app said it started at, for the clean-up.
const pids = () => [...said().matchAll(/is starting, at pid (\d+)/g)].map((m) => Number(m[1]));

/// Stop every app the run started, by the pid each start line said.
function stopAll() {
  for (const pid of pids()) {
    try {
      process.kill(pid, "SIGTERM");
    } catch {
      // Gone already.
    }
  }
}

function fail(...why) {
  process.stderr.write(`${why.join("\n")}\n\nThe app's log:\n${said()}\nThe feed was asked:\n${asked.join("\n")}\n`);
  stopAll();
  run?.app.kill("SIGTERM");
  process.exit(1);
}

/// Ask the workbench for something with the key.
async function ask(path, method = "GET") {
  return fetch(`${ORIGIN}${path}`, {
    method,
    headers: { cookie: `workbench_key=${KEY}` },
    signal: AbortSignal.timeout(20_000),
  });
}

/// Wait until `seen` says yes, or fail saying `what`.
async function until(what, seen) {
  const started = Date.now();
  while (Date.now() - started < PATIENCE) {
    if (await seen()) {
      return;
    }
    await pause(200);
  }
  fail(`Waited ${PATIENCE / 1000} s for ${what}.`);
}

/// Say what a read met, once each time it changes.
let last;
function heard(line) {
  if (line !== last) {
    last = line;
    process.stdout.write(`${line}\n`);
  }
}

/// The Devices reading, or `undefined` while nothing answers.
async function devices() {
  try {
    const answer = await ask("/api/ui/devices");
    if (!answer.ok) {
      heard(`the Devices read answered ${answer.status}: ${await answer.text()}`);
      return undefined;
    }
    return await answer.json();
  } catch (trouble) {
    heard(`the Devices read failed: ${trouble.cause?.code ?? trouble.message}`);
    return undefined;
  }
}

/// Press Update the way the row does.
async function press() {
  process.stdout.write("Pressing Update\n");
  const pressed = await ask("/api/ui/devices/this/update", "POST");
  if (!pressed.ok) {
    fail(`The press was refused: ${pressed.status} ${await pressed.text()}`);
  }
  const reading = await pressed.json();
  if (reading.this_updating == null) {
    fail("The press was answered with no Update going.");
  }
  if (reading.this_update_trouble != null) {
    fail(`The press left the trouble line standing: ${reading.this_update_trouble}`);
  }
}

/// What the row reads, as one word: down, its Update going, its trouble line,
/// or back with neither.
function reads(now) {
  if (now === undefined) return "down";
  if (now.this_updating != null) return `updating ${now.this_updating.percent ?? ""}`.trim();
  if (now.this_update_trouble != null) return `trouble: ${now.this_update_trouble}`;
  return "back";
}

// The failing run. A start that remembers handing over to a version it isn't
// on carries the trouble line from the start.
const MISSED = "0.0.9";
launch("failing", `${FEED}corrupt/`, MISSED);

await until(`Verkstead ${LOW} to answer`, async () => (await devices()) !== undefined);

const landing = `Verkstead ${MISSED} was installed, but this device started on ${LOW}.`;
await until("the trouble line for an Update that didn't land", async () =>
  (await devices())?.this_update_trouble?.includes(landing),
);
process.stdout.write(`The row reads ${reads(await devices())}\n`);

await press();

const failing = [];
await until("the corrupt download to be the row's trouble line", async () => {
  const state = reads(await devices());
  if (failing.at(-1) !== state) {
    failing.push(state);
    process.stdout.write(`The row reads ${state}\n`);
  }
  if (state === "down") {
    fail("The server stopped answering over a download that should have failed.");
  }
  return state.startsWith("trouble: ");
});
const line = failing.at(-1);
if (!/sha512/i.test(line)) {
  fail(`The trouble line doesn't give the updater's reason: ${line}`);
}
if (!failing.some((state) => state.startsWith("updating"))) {
  fail(`The row never read updating: ${failing.join(", ")}`);
}
if (sha512(run.installed) !== run.started) {
  fail(`${run.installed} is not the image it started from.`);
}
for (const unwanted of [`Verkstead ${HIGH} is downloaded`, `Verkstead ${HIGH} is starting`]) {
  if (said().includes(unwanted)) {
    fail(`The app went on past a download that should have failed: "${unwanted}".`);
  }
}
process.stdout.write(`A corrupt image left the device on ${LOW}: ${failing.join(", ")}\n`);

stopAll();
await until("the failing run's app to stop", async () => (await devices()) === undefined);
process.stdout.write(said());

// The good run, from a fresh install.
launch("updating", FEED);

await until(`Verkstead ${LOW} to answer`, async () => (await devices()) !== undefined);

const before = await devices();
if (before.this_install !== "AppImage") {
  fail(`The server reads its install as ${before.this_install}, not AppImage.`);
}
if (before.this_update_trouble != null) {
  fail(`A fresh install reads a trouble line: ${before.this_update_trouble}`);
}
if (!said().includes(`Verkstead ${LOW} is starting`)) {
  fail(`The app's log does not say Verkstead ${LOW} started.`);
}

await press();

// Read through the restart: *updating…* with the app's reports on it, then
// nothing answering, then a server with no Update going.
const states = [];
await until("the relaunched server to answer", async () => {
  const state = reads(await devices());
  if (states.at(-1) !== state) {
    states.push(state);
    process.stdout.write(`The row reads ${state}\n`);
  }
  if (state.startsWith("trouble: ")) {
    fail(`The Update failed: ${state}`);
  }
  return state === "back" && states.includes("down");
});

const reports = states.filter((state) => /^updating \d+$/.test(state));
if (reports.length === 0) {
  fail("The app never reported how much was down.", `The row read: ${states.join(", ")}`);
}

await until(`Verkstead ${HIGH} to say it started`, async () =>
  said().includes(`Verkstead ${HIGH} is starting`),
);

const written = said();
for (const wanted of [
  `Verkstead ${HIGH} is downloaded, so the app quits and starts it`,
  "this launch replaces an app that updated itself",
]) {
  if (!written.includes(wanted)) {
    fail(`The app's log does not say "${wanted}".`);
  }
}
for (const unwanted of [
  "Verkstead is already running, so this launch hands over",
  "something is already listening on",
  "didn't land",
]) {
  if (written.includes(unwanted)) {
    fail(`The relaunch met the app it replaced, or missed its version: "${unwanted}".`);
  }
}
if (sha512(run.installed) !== offered) {
  fail(`${run.installed} is not the image the feed offered.`);
}
if ((await devices())?.this_update_trouble != null) {
  fail("The relaunch on the version handed over to reads a trouble line.");
}

process.stdout.write(
  `Verkstead ${LOW} updated itself to ${HIGH} through the feed: ${states.join(", ")}\n`,
);

// And put away: the relaunched app, which nothing here started, by the pid
// its own start line said.
stopAll();
await until("the relaunched app to stop", async () => (await devices()) === undefined);
feed.close();
process.stdout.write(said());
process.exit(0);
