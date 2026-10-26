//! This device's Update, which the app does the whole of (ADR-0027, *A device
//! updates itself*).
//!
//! **The server asks over its stdout and the app answers over the API.** A
//! press of Update on this device's Devices row has the sidecar say
//! [`ASKING`] — one whole line, which reaches the app through the same reading
//! `heard` files into the log — and the app drives electron-updater on it:
//! check, download, then `quitAndInstall`. How much is down goes back to the
//! server over the viewer's own API with the **Workbench Key** read afresh out
//! of the Data Directory, so the row's state is the server's to draw and a
//! phone sees it too. No new channel either way.
//!
//! **electron-updater stands behind [`Updater`]**, the four things of it this
//! uses, so that vitest drives the exchange against a fake: `main.ts` hands the
//! real `autoUpdater` in, and it is the only file that imports the package.
//!
//! **And the relaunch must not meet the app it replaces.** `quitAndInstall` on
//! an AppImage starts the new image before the old app has quit, and the old
//! one is still holding the single-instance lock and, for the moments its
//! sidecar takes to stop, port 8422 — so a new launch would hand over to an app
//! that is going, or put the foreign-listener dialog up over its own server.
//! The app says its own pid in [`RELAUNCH`] before handing over, the new launch
//! inherits it, and [`waitOut`] has the new one wait for that pid to be gone
//! and the address to be free before it asks for either.

import { COOKIE } from "./notices.js";

/// What the sidecar says on stdout when this device's Update is pressed — the
/// server's `updating::ASKING`, matched as the whole line.
export const ASKING = "verkstead-desktop: update this device";

/// The variable a handed-over app leaves its own pid in, for the launch that
/// replaces it to wait out — see [`waitOut`].
export const RELAUNCH = "VERKSTEAD_RELAUNCH_AFTER";

/// The variable a test points electron-updater at a local `generic` feed with,
/// in place of the GitHub Release the packed `app-update.yml` names. Read
/// nowhere else, and nothing a packed app sets on its own: unset, the app
/// updates from where it was packed to.
export const FEED = "VERKSTEAD_UPDATE_FEED";

/// The subset of electron-updater's `AppUpdater` this module drives.
export interface Updater {
  /// Ask the feed what the latest release is.
  checkForUpdates(): Promise<{
    readonly isUpdateAvailable: boolean;
    readonly updateInfo: { readonly version: string };
  } | null>;
  /// Download it, sha512 checked, while `download-progress` says how far.
  downloadUpdate(): Promise<unknown>;
  /// Quit this app, install what was downloaded, and start it again.
  quitAndInstall(isSilent?: boolean, isForceRunAfter?: boolean): void;
  /// Hear how much of the download is done.
  on(event: "download-progress", heard: (progress: { percent: number }) => void): unknown;
}

/// What the exchange needs of the app around it.
export interface Updating {
  /// The updater, set up — see [`prepare`].
  updater: Updater;
  /// Tell the server how much is down, as a whole percentage.
  report(percent: number): Promise<void>;
  /// Make ready for the quit `quitAndInstall` starts: the sidecar going is
  /// expected from here, and the new launch is to wait this one out.
  handOver(): void;
  /// Say a line in the app's log.
  say(line: string): void;
}

/// How an Update came out, short of the relaunch that is its success.
export type Outcome =
  | { readonly kind: "handed-over"; readonly version: string }
  | { readonly kind: "nothing-newer" }
  | { readonly kind: "failed"; readonly why: string };

/// Set the updater up for an Update the human pressed for: nothing downloaded
/// until asked, nothing installed on a quit nobody asked to install on, its
/// own lines in the app's log, and the test's feed where one is named.
export function prepare(
  updater: Updater & {
    autoDownload: boolean;
    autoInstallOnAppQuit: boolean;
    logger: unknown;
    setFeedURL(options: { provider: "generic"; url: string }): void;
  },
  env: Partial<Record<string, string>>,
  say: (line: string) => void,
): void {
  updater.autoDownload = false;
  updater.autoInstallOnAppQuit = false;
  updater.logger = {
    info: (message: unknown) => say(`electron-updater: ${String(message)}`),
    warn: (message: unknown) => say(`electron-updater warns: ${String(message)}`),
    error: (message: unknown) => say(`electron-updater failed: ${String(message)}`),
    debug: () => {},
  };

  const feed = env[FEED];
  if (feed !== undefined && feed !== "") {
    say(`${FEED} says ${feed}, so updates are read from that feed rather than the Release`);
    updater.setFeedURL({ provider: "generic", url: feed });
  }
}

/// Take this device onto the latest release: check, download, and hand over to
/// `quitAndInstall`, which quits this app and starts the new one.
///
/// Reports are made a whole percent at a time and never awaited in the
/// download's way: a server slow to take one is not a reason to stall the
/// bytes, and one that refused is said and carried on past.
export async function update(updating: Updating): Promise<Outcome> {
  const { updater, say } = updating;
  let reported = -1;

  const report = (percent: number): void => {
    const whole = Math.floor(percent);
    if (whole <= reported) {
      return;
    }
    reported = whole;
    updating.report(whole).catch((trouble: unknown) => {
      say(`the server could not be told the update is ${whole}% down — ${String(trouble)}`);
    });
  };

  try {
    say("the server asked for this device to be updated, so the app checks for a release");
    const checked = await updater.checkForUpdates();

    if (checked === null || !checked.isUpdateAvailable) {
      say("there is no newer release to update to");
      return { kind: "nothing-newer" };
    }

    const version = checked.updateInfo.version;
    say(`Verkstead ${version} is the latest release, so it is downloaded`);

    updater.on("download-progress", ({ percent }) => report(percent));
    report(0);
    await updater.downloadUpdate();
    report(100);

    say(`Verkstead ${version} is downloaded, so the app quits and starts it`);
    updating.handOver();
    updater.quitAndInstall(true, true);

    return { kind: "handed-over", version };
  } catch (trouble) {
    const why = trouble instanceof Error ? trouble.message : String(trouble);
    say(`this device could not be updated — ${why}`);
    return { kind: "failed", why };
  }
}

/// Tell the server how much of the release is down, with the key read afresh
/// — [`keyIn`](./key.js)'s answer, handed in as `secret`.
export async function told(
  url: string,
  secret: string | undefined,
  percent: number,
): Promise<void> {
  const answer = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(secret === undefined ? {} : { cookie: `${COOKIE}=${secret}` }),
    },
    body: JSON.stringify({ percent }),
    signal: AbortSignal.timeout(5_000),
  });

  if (!answer.ok) {
    throw new Error(`the server answered ${answer.status}`);
  }
}

/// How long a relaunch waits for the app it replaces before going on anyway,
/// and how often it looks.
const PATIENCE = 30_000;
const EVERY = 100;

/// What a relaunch is waiting for, handed in so that vitest can say it.
export interface Replacing {
  /// Whether the process with this pid is still there.
  alive(pid: number): boolean;
  /// Whether anything is listening on the app's address.
  taken(): Promise<boolean>;
  /// The bound and the gap, for the tests.
  patience?: number;
  every?: number;
}

/// Whether a process is there to be signalled, which is the whole of what
/// signal 0 asks: `ESRCH` is gone, and `EPERM` is somebody else's process
/// holding a reused pid — not the app being waited on, so gone as well.
export function alive(pid: number): boolean {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/// Wait out the app this launch replaces, where it replaces one: until its pid
/// has gone and the address its sidecar held is free. Says how long it waited,
/// or `undefined` where there was nothing to wait for. Bounded, so a pid that
/// never goes is a launch that goes on and meets whatever it meets.
export async function waitOut(
  env: Partial<Record<string, string>>,
  replacing: Replacing,
): Promise<number | undefined> {
  const said = env[RELAUNCH];
  const pid = said === undefined ? NaN : Number.parseInt(said, 10);
  if (!Number.isInteger(pid) || pid <= 0 || pid === process.pid) {
    return undefined;
  }

  const patience = replacing.patience ?? PATIENCE;
  const every = replacing.every ?? EVERY;
  const started = Date.now();
  const pause = () => new Promise<void>((done) => setTimeout(done, every));

  while (Date.now() - started < patience && replacing.alive(pid)) {
    await pause();
  }
  while (Date.now() - started < patience && (await replacing.taken())) {
    await pause();
  }

  return Date.now() - started;
}
