//! The **Install Offer**: whether a launch puts it up, what it says, and what
//! pressing **Install** comes to.
//!
//! **A packed app running from outside an Applications folder offers to install
//! itself** (ADR-0020, *Amended: a download offers to install itself*). On Linux
//! that is `$APPIMAGE` not under `~/Applications` — which is where AppImageLauncher
//! and Gear Lever put theirs too, so a human who already integrated the file is
//! left alone. One rule rather than Downloads by path: a human who keeps their
//! AppImages in `~/bin` sees the offer once and ticks **Don't ask again**.
//!
//! On a Mac it is Electron's own `isInApplicationsFolder` answering no — which
//! it does from the dmg, from Downloads, from the Desktop and from the path
//! Gatekeeper translocates a quarantined app to — and **Install there is
//! Electron's `moveToApplicationsFolder`** into `/Applications`, which moves the
//! bundle, trashes a copy that was there, asks for the administrator's password
//! where the folder wants one, and quits and relaunches on its own. Neither call
//! can be made from a module vitest runs, so the first is handed in as a value
//! and the second as a function — see [`Placing`] and [`moveInto`].
//!
//! **Never on a checkout run and never on a hidden start**, each said for its
//! own reason — see [`offered`]. A login's start has nobody at a window that
//! never came up, so the offer waits for the next launch somebody is at. And
//! never on Windows, whose download is the installer.
//!
//! **Install on Linux is a move, an entry and the icons.** `$APPIMAGE` goes to
//! `~/Applications/Verkstead-x86_64.AppImage` — the release's own name, whatever
//! a browser renamed the download to — and stays executable; a desktop entry
//! naming the copy goes beside the other applications'; and the icon set is
//! copied out of the mounted image, because an entry whose `Icon` names nothing
//! draws the generic launcher icon and the only icons are inside an image that
//! is gone once the app is not running from it. Move rather than copy: a
//! download left behind is one that shows the offer again if it is ever run.
//!
//! **A Verkstead already installed has its version read first** — see
//! [`version`] and [`newer`]. A newer one is offered to open instead, and an
//! older or the same one is replaced, which is what both Installs already do.
//!
//! **Everything that decides a path or a file's text is a function of values**,
//! which vitest runs with the values handed in — the same shape
//! [`startup.ts`](./startup.js) has. What is left for the edge is the moving and
//! the writing, which is [`install`], and the dialog and the relaunch, which are
//! `main.ts`'s.

import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import type { Placement } from "./bridge.js";
import { absolute, dataHome, type Machine } from "./platform.js";
import type { Settings } from "./settings.js";
import { APP_ID, COMMENT, HIDDEN, NAME, quoted } from "./startup.js";
import { unmounted } from "./unmounted.js";

/// The flag that is the dialog's **Install** with nobody asked: outside
/// Applications it installs and relaunches, inside it says so and carries on.
/// An argument of the app's own beside [`HIDDEN`](./startup.js), and how the
/// release legs install the way a downloader does — a native dialog cannot be
/// pressed on a runner.
export const INSTALL = "--install";

/// The folder the offer installs into, under the human's home.
export const APPLICATIONS = "Applications";

/// What the installed file is called: the release's own name, which is what
/// `electron-builder.yml`'s `artifactName` says and what a browser may have
/// renamed on the way down.
export const FILENAME = "Verkstead-x86_64.AppImage";

/// What the installed copy's icons are called, and so what its entry's `Icon`
/// names: the app id, which is what the autostart entry's `Icon` already names —
/// so the one icon set draws both.
export const ICON = APP_ID;

/// Where the mounted image keeps its icon set, relative to `$APPDIR` —
/// electron-builder's own layout, `<size>/apps/<executable>.png`.
export const HICOLOR = join("usr", "share", "icons", "hicolor");

/// The fields `electron-builder.yml` gives the packed entry, said again here
/// because the installed entry is written by this app rather than by the pack.
/// `offer.test.ts` reads the yml for both, so the two cannot drift apart.
export const CATEGORIES = "Development;";
export const KEYWORDS = "agent;coding;sessions;workbench;";

/// What a run is, as far as the offer is concerned.
///
/// Values rather than reads of the running application — the same shape
/// [`Registering`](./startup.js) is.
export interface Placing extends Machine {
  /// Whether this is a packed app rather than a checkout run: `app.isPackaged`.
  packaged: boolean;

  /// The executable running: `process.execPath`, which on a Mac is inside the
  /// bundle Install moves.
  exe: string;

  /// Whether the bundle is in an Applications folder:
  /// `app.isInApplicationsFolder()`, which Electron answers on a Mac alone and
  /// which is read on a Mac alone — false elsewhere, where nothing asks it.
  applied: boolean;
}

/// Where this run is, and whether there is anything to install.
export type Placed =
  /// A file running from outside Applications, which Install would move.
  | { readonly outside: string }
  /// A file already in Applications, which there is nothing to do about.
  | { readonly inside: string }
  /// Nothing to install at all, and why — said in the log.
  | { readonly nowhere: string };

/// Where the app installs to on this machine: `~/Applications`, or nothing
/// where there is no home — read by the rule the autostart directory is read
/// by, a leading `/` or nothing.
export function applications({ env }: Machine): string | undefined {
  const home = absolute(env.HOME);
  return home === undefined ? undefined : join(home, APPLICATIONS);
}

/// Where this run is, out of what it is.
export function placed(placing: Placing): Placed {
  // Before the platform is asked anything, for the reason `startup.ts`'s
  // `where` gives: a checkout run is the dev shell's Electron, and there is no
  // file of the human's to move.
  if (!placing.packaged) {
    return { nowhere: "this is a checkout run, so there is nothing to install" };
  }

  if (placing.platform === "win32") {
    return { nowhere: "the Windows download is the installer, so there is nothing to install" };
  }

  if (placing.platform === "darwin") {
    const app = bundle(placing.exe);
    return placing.applied ? { inside: app } : { outside: app };
  }

  if (placing.platform !== "linux") {
    return { nowhere: `there is no Install Offer on ${placing.platform}` };
  }

  const running = absolute(placing.env.APPIMAGE);
  if (running === undefined) {
    return { nowhere: "this run is not an AppImage, so there is no file to install" };
  }

  const dir = applications(placing);
  if (dir === undefined) {
    return { nowhere: "this machine names no home, so there is nowhere to install to" };
  }

  return running.startsWith(`${dir}/`) ? { inside: running } : { outside: running };
}

/// The bundle a Mac executable at `exe` is inside —
/// `<bundle>/Contents/MacOS/<name>` — or the executable itself where it is in
/// no bundle, which is only ever a log line's worth of difference.
export function bundle(exe: string): string {
  const parts = exe.split("/");
  return parts.length > 3 && parts.at(-2) === "MacOS" && parts.at(-3) === "Contents"
    ? parts.slice(0, -3).join("/")
    : exe;
}

/// Where this run is from, as the Desktop page's Install row is drawn from it:
/// outside, and the path written the way the dialog writes it, or not outside
/// at all — an app already installed, a checkout run, and a platform with no
/// offer alike.
///
/// **A Mac's names no path**, for the reason [`asking`] gives: a quarantined
/// app runs from the randomised folder Gatekeeper translocated it to, which is
/// what the macos-15 probe drew on the row.
export function placement(where: Placed, machine: Machine): Placement {
  if (!("outside" in where)) {
    return { outside: false };
  }

  return machine.platform === "darwin" ? { outside: true } : { outside: true, from: shown(where.outside, machine) };
}

/// Whether this launch puts the offer up: the file it would install, or why it
/// does not.
export type Offer = { readonly offer: string } | { readonly why: string };

/// Whether the offer is made, given where this run is, whether it is a hidden
/// start — what [`hidden`](./startup.js) already answers, asked only of a run
/// that is outside Applications, so that a platform with no offer is never put
/// the question — and the settings.
///
/// **Each no is said for its own reason**, because the log is how somebody
/// works out why a download did not ask: a checkout, a login's start, an app
/// already installed and a box ticked are four different answers to that.
export function offered(where: Placed, unseen: () => boolean, chosen: Settings): Offer {
  if ("nowhere" in where) {
    return { why: where.nowhere };
  }

  if ("inside" in where) {
    return { why: `it already runs from ${where.inside}` };
  }

  if (unseen()) {
    return { why: "this launch is a login's, so the offer waits for one somebody is at" };
  }

  if (!chosen.installOffer) {
    return { why: "the desktop settings say don't ask again" };
  }

  return { offer: where.outside };
}

/// One file to copy, from the mounted image to the human's icon theme.
export interface Copy {
  readonly from: string;
  readonly to: string;
}

/// What Install does, as paths and text.
export interface Plan {
  /// The file running, which is moved.
  readonly from: string;

  /// The folder it goes into, made where it is not there.
  readonly dir: string;

  /// And the file it becomes.
  readonly file: string;

  /// The desktop entry, and what is written in it.
  readonly entry: string;
  readonly text: string;

  /// The icons, copied out of the mount.
  readonly icons: readonly Copy[];
}

/// What Install does for the file at `from`, on this machine, with the image
/// mounted at `appdir` holding `found` under its [`HICOLOR`] tree.
///
/// Nothing where there is no home or no data home to install into, which
/// [`placed`] has already ruled out by the time this is asked.
export function plan(
  from: string,
  machine: Machine,
  appdir: string | undefined,
  found: readonly string[],
): Plan | undefined {
  const dir = applications(machine);
  const data = dataHome(machine);
  if (dir === undefined || data === undefined) {
    return undefined;
  }

  const file = join(dir, FILENAME);

  return {
    from,
    dir,
    file,
    entry: join(data, "applications", `${APP_ID}.desktop`),
    text: entry(file),
    icons: appdir === undefined ? [] : icons(join(appdir, HICOLOR), found, join(data, "icons", "hicolor")),
  };
}

/// The installed entry, for the AppImage at `file`.
///
/// **The packed entry's fields, naming the copy.** `Name`, `Comment`,
/// `Categories` and `Keywords` are what `electron-builder.yml` gives the entry
/// inside the image; `StartupWMClass` is the app id, which is what Electron
/// reports as the window's own (`desktopName` in `package.json`), so the window
/// groups under this entry rather than as some other application; and the path
/// is quoted the way the autostart entry quotes its own — see
/// [`quoted`](./startup.js). `%U` because that is what electron-builder's own
/// `Exec` ends in.
export function entry(file: string): string {
  return (
    "[Desktop Entry]\n" +
    "Type=Application\n" +
    `Name=${NAME}\n` +
    `Comment=${COMMENT}\n` +
    `Exec=${quoted(file)} %U\n` +
    `Icon=${ICON}\n` +
    "Terminal=false\n" +
    `StartupWMClass=${APP_ID}\n` +
    `Categories=${CATEGORIES}\n` +
    `Keywords=${KEYWORDS}\n`
  );
}

/// The icons to copy: every `<size>/apps/<name>.png` (or `.svg`) under the
/// image's tree at `tree`, as `found` lists them relative to it, to the same
/// size under `theme` — renamed to [`ICON`], which is what the entry names.
///
/// Anything else in the tree is not an application icon and is left where it
/// is.
export function icons(tree: string, found: readonly string[], theme: string): Copy[] {
  const copies: Copy[] = [];

  for (const relative of found) {
    const parts = relative.split("/");
    if (parts.length !== 3 || parts[1] !== "apps") {
      continue;
    }

    const extension = /\.(png|svg)$/.exec(parts[2] ?? "")?.[0];
    if (extension === undefined) {
      continue;
    }

    copies.push({
      from: join(tree, relative),
      to: join(theme, parts[0] ?? "", "apps", `${ICON}${extension}`),
    });
  }

  return copies;
}

/// `path` as a human reads it in a sentence: under the home, written from `~`.
export function shown(path: string, { env }: Machine): string {
  const home = absolute(env.HOME);
  return home !== undefined && (path === home || path.startsWith(`${home}/`))
    ? `~${path.slice(home.length)}`
    : path;
}

/// The arguments the installed copy is relaunched with: this launch's own,
/// past the executable, without [`INSTALL`] — the copy is installed, and asked
/// again it would only say so — and without [`HIDDEN`](./startup.js).
///
/// **A relaunch is always one somebody is at.** The launch's offer is never
/// made on a hidden start, and an Install pressed on the Desktop page, or an
/// **Open installed**, is pressed in a window: a login's `--hidden` carried
/// over would bring the copy up in the tray alone, which reads as the app
/// having quit.
export function relaunched(argv: readonly string[]): string[] {
  return argv.slice(1).filter((argument) => argument !== INSTALL && argument !== HIDDEN);
}

/// What the dialog says, and the native message box's own fields for it.
export interface Asking {
  readonly message: string;
  readonly detail: string;
  readonly buttons: string[];
  readonly defaultId: number;
  readonly cancelId: number;
  readonly checkboxLabel: string;
}

/// Where in [`Asking.buttons`] **Install** is.
export const INSTALLING = 0;

/// The offer, for the file at `from`: the question, a sentence naming where it
/// runs from and what installing does, **Install** against **Skip**, and the
/// box. The words are ADR-0020's.
///
/// **A Mac's sentence names no path**: the one a translocated app runs from is
/// a randomised folder nobody chose, and Applications is the one place the
/// human knows by name.
///
/// **Pressed on the Desktop page, it says one more sentence** where sessions
/// are running on this device: `stopping`, the page's own words for them, read
/// the way the Update card reads them — the install quits the app and the
/// server with it. With none, the dialog is the launch's own.
export function asking(from: string, machine: Machine, stopping?: string): Asking {
  const into = applications(machine);
  const more = stopping === undefined || stopping === "" ? "" : ` ${stopping}`;

  return {
    message: "Install Verkstead?",
    detail:
      (machine.platform === "darwin"
        ? "Verkstead is running from outside Applications. " +
          "Installing moves it into Applications and opens it from there."
        : `Verkstead is running from ${shown(dirname(from), machine)}. ` +
          `Installing moves it to ${into === undefined ? APPLICATIONS : shown(into, machine)}, ` +
          "adds it to your applications menu, and restarts it from there.") + more,
    buttons: ["Install", "Skip"],
    defaultId: INSTALLING,
    cancelId: 1,
    checkboxLabel: "Don't ask again",
  };
}

/// What the failure dialog says: why the install did not happen, and that the
/// app carries on from where it is.
export function failure(why: string, from: string, machine: Machine): { title: string; content: string } {
  return {
    title: "Verkstead couldn't be installed.",
    content: `${why}\n\nIt carries on running from ${
      machine.platform === "darwin" ? "outside Applications" : shown(dirname(from), machine)
    }.`,
  };
}

/// An install that stopped part of the way, and the sentence a human is shown
/// for it — the step that failed, with what the system said.
export class NotInstalled extends Error {}

/// A Mac's Install: `move`, which is `app.moveToApplicationsFolder`, made and
/// its answer read.
///
/// **True is the app on its way out**: Electron has moved the bundle and is
/// quitting this process to relaunch the copy. **False is the password dialog
/// cancelled**, and a throw is a move that could not be made — both a
/// [`NotInstalled`] worded for the failure dialog rather than a silent skip
/// (ADR-0020): the human pressed Install.
export function moveInto(move: () => boolean): void {
  let moved: boolean;
  try {
    moved = move();
  } catch (trouble) {
    const said = trouble instanceof Error ? trouble.message : String(trouble);
    throw new NotInstalled(`It couldn't be moved into Applications (${said}).`);
  }

  if (!moved) {
    throw new NotInstalled("Moving it into Applications was cancelled.");
  }
}

/// Do what `plan` says: the icons, the move, then the entry.
///
/// **In that order, so that a failure leaves as little as it can.** The icons
/// are harmless on their own; a move that fails leaves the download where it
/// was and no entry naming a file that is not there; and an entry that could
/// not be written after the move leaves an app in Applications that the next
/// launch finds installed. Every failure is a [`NotInstalled`] worded for the
/// dialog.
export function install(plan: Plan, machine: Machine): void {
  const step = (what: string, act: () => void): void => {
    try {
      act();
    } catch (trouble) {
      const said = trouble instanceof Error ? trouble.message : String(trouble);
      throw new NotInstalled(`It couldn't ${what} (${said}).`);
    }
  };

  const into = shown(plan.dir, machine);

  step("copy its icons", () => {
    for (const icon of plan.icons) {
      mkdirSync(dirname(icon.to), { recursive: true });
      copyFileSync(icon.from, icon.to);
    }
  });

  step(`make ${into}`, () => mkdirSync(plan.dir, { recursive: true }));

  step(`be moved to ${into}`, () => {
    move(plan.from, plan.file);
    chmodSync(plan.file, 0o755);
  });

  step("be added to your applications menu", () => {
    mkdirSync(dirname(plan.entry), { recursive: true });
    writeFileSync(plan.entry, plan.text, "utf8");
  });
}

/// Move `from` to `to`: a rename where the two are on one filesystem, and a
/// copy and a removal where they are not — a Downloads on another disk than the
/// home is ordinary.
///
/// The running image is mounted from the file, which the mount holds open, so
/// the app goes on running from it whichever of the two this was.
function move(from: string, to: string): void {
  try {
    renameSync(from, to);
  } catch (trouble) {
    if ((trouble as NodeJS.ErrnoException).code !== "EXDEV") {
      throw trouble;
    }
    copyFileSync(from, to);
    rmSync(from);
  }
}

/// Where a Mac's Install puts the bundle: `moveToApplicationsFolder`'s own
/// destination, the bundle's name in `/Applications`.
export const MAC_INSTALLED = "/Applications/Verkstead.app";

/// The installed copy Install would replace on this machine, or nothing where
/// there is nowhere to install to.
export function installed(machine: Machine): string | undefined {
  if (machine.platform === "darwin") {
    return MAC_INSTALLED;
  }

  const dir = applications(machine);
  return dir === undefined ? undefined : join(dir, FILENAME);
}

/// What starts the installed copy at `at`: the bundle's executable on a Mac —
/// the product name, under `Contents/MacOS` — and the AppImage itself on Linux.
export function executable(at: string, platform: NodeJS.Platform): string {
  return platform === "darwin" ? join(at, "Contents", "MacOS", NAME) : at;
}

/// The version a Mac bundle's `Info.plist` says it is:
/// `CFBundleShortVersionString`, which electron-builder writes from
/// `package.json`'s version.
///
/// Read the way [`launchd.ts`](./launchd.js) reads its two keys — the string
/// after the key — rather than by a plist parser.
export function shortVersion(plist: string): string | undefined {
  const after = plist.split("<key>CFBundleShortVersionString</key>")[1];
  const said = after === undefined ? undefined : /^\s*<string>([^<]*)<\/string>/.exec(after)?.[1]?.trim();
  return said === "" ? undefined : said;
}

/// The version an AppImage's root desktop entry says it is:
/// `X-AppImage-Version`, which electron-builder writes into it. The Rust
/// AppImage's entry had the same name and no such key, so it says nothing.
export function imageVersion(desktop: string): string | undefined {
  const said = /^X-AppImage-Version=(.*)$/m.exec(desktop)?.[1]?.trim();
  return said === "" ? undefined : said;
}

/// The version of the copy installed at `at`, or nothing where it cannot be
/// read — which [`newer`] counts as older.
///
/// **A Mac's is a file in the bundle.** **Linux's is inside the image**, and
/// the AppImage runtime extracts that one file with `--appimage-extract`
/// without starting Electron — so every release ever shipped answers, and
/// nothing of the copy's own runs. It extracts into the directory it is run
/// from, so it is run from a temporary one of this app's own, which goes
/// afterwards; and with the environment this run was given rather than its
/// mount's, as the sidecar is (see [`unmounted`]).
export function version(at: string, machine: Machine): string | undefined {
  if (machine.platform === "darwin") {
    const plist = readable(join(at, "Contents", "Info.plist"));
    return plist === undefined ? undefined : shortVersion(plist);
  }

  let scratch: string | undefined;
  try {
    scratch = mkdtempSync(join(absolute(machine.env.TMPDIR) ?? tmpdir(), "verkstead-installed-"));
    const name = `${APP_ID}.desktop`;
    spawnSync(at, ["--appimage-extract", name], {
      cwd: scratch,
      env: unmounted(machine.env),
      stdio: "ignore",
      timeout: 10_000,
    });
    const desktop = readable(join(scratch, "squashfs-root", name));
    return desktop === undefined ? undefined : imageVersion(desktop);
  } catch {
    return undefined;
  } finally {
    if (scratch !== undefined) {
      rmSync(scratch, { recursive: true, force: true });
    }
  }
}

/// A file's text, or nothing where it cannot be read.
function readable(file: string): string | undefined {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return undefined;
  }
}

/// A version as semver orders it: the three numbers, and the pre-release's
/// identifiers. Build metadata orders nothing, so it is dropped.
interface Semver {
  readonly core: readonly [number, number, number];
  readonly pre: readonly string[];
}

function semver(text: string): Semver | undefined {
  const parts = /^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/.exec(text.trim());
  if (parts === null) {
    return undefined;
  }

  return {
    core: [Number(parts[1]), Number(parts[2]), Number(parts[3])],
    pre: parts[4] === undefined ? [] : parts[4].split("."),
  };
}

/// How `a` orders against `b`: negative, zero or positive.
///
/// **A pre-release is older than its release**, and pre-releases order
/// identifier by identifier — numbers numerically, numbers before words, words
/// as text, and the shorter first where one runs out.
function order(a: Semver, b: Semver): number {
  for (let i = 0; i < 3; i++) {
    const by = (a.core[i] ?? 0) - (b.core[i] ?? 0);
    if (by !== 0) {
      return by;
    }
  }

  if (a.pre.length === 0 || b.pre.length === 0) {
    return b.pre.length - a.pre.length;
  }

  for (let i = 0; i < Math.min(a.pre.length, b.pre.length); i++) {
    const x = a.pre[i] ?? "";
    const y = b.pre[i] ?? "";
    const numeric = /^\d+$/;
    if (numeric.test(x) && numeric.test(y)) {
      const by = Number(x) - Number(y);
      if (by !== 0) {
        return by;
      }
    } else if (numeric.test(x) !== numeric.test(y)) {
      return numeric.test(x) ? -1 : 1;
    } else if (x !== y) {
      return x < y ? -1 : 1;
    }
  }

  return a.pre.length - b.pre.length;
}

/// Whether the installed copy, at version `installed`, is newer than the one
/// `running`.
///
/// **Equal is not newer**, and **a version that cannot be read is older**:
/// the one being run is what the human has in hand, so Install goes ahead.
export function newer(installed: string | undefined, running: string): boolean {
  const there = installed === undefined ? undefined : semver(installed);
  const here = semver(running);
  return there !== undefined && here !== undefined && order(there, here) > 0;
}

/// Where in [`instead`]'s buttons **Open installed** is.
export const OPENING = 0;

/// What the open-installed dialog says, over a newer copy at `installed` than
/// the one `running`. The words are ADR-0020's.
export function instead(
  installed: string,
  running: string,
): Omit<Asking, "checkboxLabel"> {
  return {
    message: "Open the installed Verkstead?",
    detail: `Verkstead ${installed} is already installed, and this is ${running}.`,
    buttons: ["Open installed", "Keep this one"],
    defaultId: OPENING,
    cancelId: 1,
  };
}
