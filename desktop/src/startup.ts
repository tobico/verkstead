//! **Launch on Startup**: whether Verkstead comes up when the machine's desktop
//! session does.
//!
//! **The platform's own registration is the state**, and nothing duplicates it
//! (ADR-0020, Set 847 Q12a). There is no entry for this in the app's settings
//! file and none is added: the box on the Desktop page is drawn from reading the
//! registration itself, checking it writes one and unchecking it takes it away.
//! A human who turns it off with their desktop's own settings has unchecked the
//! box, and Verkstead agrees with them rather than argues.
//!
//! **On Linux that registration is the Rust tray app's own file** — an XDG
//! autostart entry named for the app id, in the same directory under the same
//! name — which is how the old registration is taken over rather than doubled.
//! So the readings carry over exactly as `crates/desktop/src/startup/xdg.rs`
//! makes them, because a human may have edited that file: the two keys a desktop
//! turns an entry off with, the specification's own reading of
//! `$XDG_CONFIG_HOME`, the path quoted whole, and `$APPIMAGE` in preference to
//! the running executable. **What does not carry over is the command**: that
//! entry said a verb of the CLI and `--no-open`, and this app has neither — it
//! is its own way in, and what it writes instead is [`HIDDEN`], a flag of its
//! own saying come up with no window.
//!
//! **On a Mac and on Windows it is Electron's login-item API**, which is a call
//! rather than a file — the app is always a bundle now, which is what the tray
//! app's hand-written plist was working around. Those two arms are written here
//! and proven in stages 06 and 07; the registration is still the state there,
//! the box being drawn from reading it back through the same call.
//!
//! **And nowhere at all on an unpackaged run**, whatever the platform (Q2). What
//! a registration could name from a checkout is the dev shell's Electron in the
//! nix store plus a build directory, which breaks the next time either moves —
//! so the whole path is written and tested, and the writing is refused where
//! there is nothing worth writing. The box is greyed with the reason on it,
//! which is the same shape as a machine that names no configuration directory to
//! put an entry in.
//!
//! **Every launch rewrites the registration while it is there** — see
//! [`Startup.refresh`] — as the tray app did: an app that was moved leaves a
//! registration naming a path that is no longer anything, and the next launch by
//! hand is the moment that heals. **Unregistered stays unregistered**: nothing
//! here ever writes a registration that was not already asked for.
//!
//! **A login start comes up hidden while the tray is shown, and shown
//! otherwise** — see [`hidden`] — which is the reading `--no-open` made of a
//! login for the tray app rather than a **Launch on Startup** that needs the
//! tray: an app with no icon and no window is a Verkstead nobody can reach.
//!
//! **And a login start is said two ways, because the platforms say it two ways.**
//! Linux's `Exec` line and Windows' Run key are command lines, so both carry
//! [`HIDDEN`] and [`hidden`] reads it off the arguments. A Mac's login item
//! carries no arguments — `args` is Windows' alone — and `openAsHidden`, which
//! was that platform's own word for coming up with no window, has been
//! deprecated and inert since macOS 13. So a Mac is asked instead whether the
//! login item started this run, which is [`Startup.atLogin`]; nothing asks for
//! `openAsHidden` any more, and the window is kept off the screen by this app
//! rather than by the platform, which is what it already was on the other two.
//!
//! All of it is path, text and three injected calls, so every arm is an ordinary
//! unit test on this Linux runner — the login-item API arrives as [`LoginItem`]
//! rather than as a reach into `electron`, for the reason the wall in
//! `eslint.config.js` gives.

import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import { say } from "./log.js";
import { absolute, type Machine } from "./platform.js";
import type { Settings } from "./settings.js";

/// The id every registration Verkstead makes with a platform is named for, and
/// the one `crates/desktop/src/lib.rs` has held all along — the autostart entry
/// is that app's file, so the name has to be that app's name.
export const APP_ID = "net.tobico.Verkstead";

/// The flag a login start carries, on the two platforms whose registration is a
/// command line — Linux's `Exec` line and Windows' Run key. A Mac's login item
/// carries no arguments and is asked instead, which is
/// [`LoginItem.openedAtLogin`].
///
/// The app's own rather than the CLI's — this is not `--no-open`, which told the
/// tray app to hand nobody a browser, and there is no verb in front of it. What
/// it means here is that the window stays off the screen while there is an icon
/// to reach the app by; [`hidden`] is the whole of that reading.
export const HIDDEN = "--hidden";

/// Where autostart entries go under a configuration directory.
const AUTOSTART = "autostart";

/// What the entry calls Verkstead, which is what a Startup Applications list
/// draws beside its checkbox — and what the tray app's entry called it.
const NAME = "Verkstead";

/// And what it says underneath, where the desktop shows one.
const COMMENT = "The workbench, in the system tray";

/// Why there is nothing worth registering from a checkout.
///
/// Worded for the human reading it under a greyed box, which is where it is
/// drawn: what is wrong is the app rather than their machine.
const UNPACKAGED =
  "Launch on Startup needs an installed Verkstead. This one is running from a " +
  "checkout, and a registration made here would name a build that will not be " +
  "there the next time the machine starts.";

/// And why a machine may have nowhere to keep one — the tray app's own wording,
/// for the same situation.
const NOWHERE =
  "Verkstead has nowhere to keep a startup registration on this machine, so it " +
  "cannot start itself with your desktop session.";

/// What a registration is a function of.
///
/// Values rather than reads of the running application, so that the two arms
/// this Linux runner will never take are still arms its tests call — the same
/// shape [`Install`](./cli.js) and [`Machine`](./platform.js) are here for.
export interface Registering extends Machine {
  /// Whether this is a packed app rather than a checkout run: `app.isPackaged`.
  packaged: boolean;

  /// The executable a registration names — `process.execPath`, which in a packed
  /// app is the app's own launcher. What `$APPIMAGE` says is preferred to it, for
  /// the reason [`named`] gives.
  exe: string;
}

/// Where Verkstead's registration goes on this machine, or why there is nowhere
/// for one.
export type Where =
  /// The XDG autostart entry at this path, which is Linux's.
  | { readonly entry: string }
  /// Electron's login-item API, which is what the other two have.
  | { readonly login: true }
  /// Nowhere, and this is what the greyed box says — about the app on an
  /// unpackaged run, and about the machine where it names no configuration
  /// directory.
  | { readonly nowhere: string };

/// How **Launch on Startup** stands, as the page draws it.
export interface Registration {
  /// Whether this machine can be registered with at all. A box that will not
  /// take a tick rather than one that ticks and does nothing.
  readonly possible: boolean;

  /// Whether Verkstead comes up with the desktop session, read from the
  /// registration itself.
  readonly on: boolean;

  /// Why it cannot be, where it cannot — the note the greyed box carries.
  readonly why?: string;

  /// What refused a registration that was asked for, where this is the answer to
  /// one.
  ///
  /// **Said rather than thrown.** The page draws what is true — a set that wrote
  /// nothing leaves the box where it was — and the reason it wrote nothing is
  /// worth a line under it rather than an invoke's own wording about a remote
  /// method.
  readonly refused?: string;
}

/// The arguments a login start is registered with, on the two platforms whose
/// registration is a call — the flag, exactly as the Linux entry's `Exec` line
/// writes it, and nothing else.
///
/// **One list rather than two, because Electron compares them.** Asking whether
/// the app opens at login is asking about a *command line* on Windows, so a read
/// that named different arguments from the write answers `false` about a
/// registration this app had just made — see [`LoginItem.registered`]. Both
/// calls are handed this same value, and there is nowhere for the two of them to
/// disagree.
///
/// **And Windows is the one of the two that reads them at all.** A Mac's login
/// item carries no arguments, which is why [`Startup.atLogin`] exists; they are
/// sent there all the same, Electron ignoring them on that platform, because one
/// value sent to both is one less thing to keep in step.
export const ARGS: string[] = [HIDDEN];

/// What Electron's `app.setLoginItemSettings` is told, on the two platforms that
/// have one.
export interface LoginAsked {
  /// Whether the app opens at login at all.
  openAtLogin: boolean;

  /// And the arguments the login start carries, which is how Windows is told to
  /// come up with no window — [`ARGS`], rather than anything worked out here.
  args: string[];
}

/// Electron's login-item API, as the two calls this needs of it.
///
/// Injected rather than imported, for the reason the wall in `eslint.config.js`
/// gives: `app` is `main.ts`'s, and everything that decides anything is a
/// function of values vitest can hand in.
export interface LoginItem {
  /// Whether the app is registered to open at login with `args` —
  /// `app.getLoginItemSettings({ args }).openAtLogin`.
  ///
  /// **The arguments are half the question rather than a detail of it.** On
  /// Windows a registration is a command line under the Run key, and Electron
  /// answers `openAtLogin` by comparing that line against the executable and
  /// the arguments it was *asked* about — which it defaults to none. So a read
  /// that left them out says Verkstead does not start with the session while it
  /// does: a box that springs back the moment it is ticked, and a registration
  /// [`Startup.refresh`] never rewrites. They are taken here rather than known
  /// at the far end for that reason — what was written and what is read back are
  /// one value.
  registered(args: string[]): boolean;

  /// Whether *this launch* is the login item's own doing —
  /// `app.getLoginItemSettings().wasOpenedAtLogin`.
  ///
  /// **A Mac's question, and only a Mac's.** The other two register a command
  /// line, so [`HIDDEN`] reaches `process.argv` and the flag is the answer; a
  /// Mac's login item carries no arguments, and the word that platform had for
  /// coming up hidden — `openAsHidden` — has been deprecated and inert since
  /// macOS 13. So this is what a Mac has instead: the platform saying the login
  /// item is what started this run, which [`hidden`] reads exactly as it reads
  /// the flag.
  openedAtLogin(): boolean;

  /// Register or unregister — `app.setLoginItemSettings`.
  register(asked: LoginAsked): void;
}

/// Verkstead's startup registration on this machine: read, written, and
/// rewritten at every launch.
///
/// Where it goes is worked out once, because that does not move while the app
/// runs; what it *says* is read from the platform every time it is asked,
/// because that does.
export interface Startup {
  /// How it stands, which is what the Desktop page draws.
  standing(): Registration;

  /// Register or unregister, and say how it stands afterwards.
  ///
  /// The whole of what checking and unchecking the box does: no settings file is
  /// touched, the registration being the state rather than a copy of it.
  set(on: boolean): Registration;

  /// Rewrite the registration with this app's own path, while there is one.
  ///
  /// Called at every launch. A registration naming an app that has moved heals
  /// itself here; an unregistered machine stays unregistered, because what this
  /// rewrites is what somebody asked for rather than something it decided for
  /// them.
  ///
  /// **Nothing here fails.** A registration that could not be rewritten is the
  /// one that was already there, which is what the launch before this one left
  /// — and no reason to stop a Verkstead that is otherwise about to serve.
  refresh(): void;

  /// Whether this launch is the registration's own doing, as the platform says
  /// rather than as the command line does.
  ///
  /// `false` everywhere the command line is the answer, which is Linux and
  /// Windows: both registrations carry [`HIDDEN`], and [`hidden`] reads it off
  /// the arguments. A Mac is where this is the whole of the answer — see
  /// [`LoginItem.openedAtLogin`].
  atLogin(): boolean;
}

/// Where this machine keeps the registration, and what may be done to it.
export function startup(registering: Registering, login: LoginItem): Startup {
  const put = where(registering);

  /// Whether Verkstead starts with the session, asked of the registration.
  ///
  /// An entry that cannot be read at all is read as no registration: what this
  /// answers is whether Verkstead *is* started by it, and one nothing here can
  /// read is one nothing here can answer for.
  const on = (): boolean => {
    if ("nowhere" in put) {
      return false;
    }
    if ("login" in put) {
      return login.registered(ARGS);
    }

    const written = read(put.entry);
    return written !== undefined && saysOn(written);
  };

  const standing = (): Registration =>
    "nowhere" in put
      ? { possible: false, on: false, why: put.nowhere }
      : { possible: true, on: on() };

  const set = (asked: boolean): Registration => {
    if ("nowhere" in put) {
      // The box is greyed, so this is a page that should not have asked — said
      // rather than enacted, and what comes back is what is true.
      say(`Launch on Startup was asked for, and ${put.nowhere}`);
      return standing();
    }

    try {
      if ("login" in put) {
        login.register({ openAtLogin: asked, args: ARGS });
      } else if (asked) {
        write(put.entry, written(named(registering)));
      } else {
        remove(put.entry);
      }
    } catch (trouble) {
      // Worded for a human under whatever the platform said underneath, because
      // a human is who reads it: this is a line under the box on the Desktop
      // page.
      const refused =
        (asked
          ? "Verkstead could not ask this machine to start it with your desktop session"
          : "Verkstead could not ask this machine to stop starting it with your desktop session") +
        ` — ${String(trouble)}`;

      say(refused);
      return { ...standing(), refused };
    }

    return standing();
  };

  return {
    standing,
    set,
    refresh: () => {
      if (on()) {
        // Whatever it answers: a rewrite that failed has already said so, and
        // the registration that stands is the one the last launch left.
        set(true);
      }
    },

    // Asked of a Mac and of nothing else: the other two carry the flag on the
    // command line, and a machine with no registration to have been started by
    // was not started by one.
    atLogin: () =>
      "login" in put && registering.platform === "darwin" && login.openedAtLogin(),
  };
}

/// Where Verkstead's registration goes, out of what this run is.
export function where(registering: Registering): Where {
  // Before the platform is asked anything, because it is a fact about the app
  // rather than about the machine: a checkout run has nothing worth registering
  // on any of the three.
  if (!registering.packaged) {
    return { nowhere: UNPACKAGED };
  }

  if (registering.platform === "darwin" || registering.platform === "win32") {
    return { login: true };
  }

  // And everything else is a Unix that is not a Mac, which is where the XDG
  // autostart specification is the answer — the same reading `platform.ts`
  // gives the directories Verkstead keeps its own things in.
  const dir = autostartDir(registering);

  return dir === undefined ? { nowhere: NOWHERE } : { entry: join(dir, `${APP_ID}.desktop`) };
}

/// Where autostart entries go: `$XDG_CONFIG_HOME/autostart` where that names an
/// absolute path, and `~/.config/autostart` otherwise — the specification's own
/// reading, and the one [`platform.ts`](./platform.js) gives Verkstead's own
/// directories.
///
/// Not one of Verkstead's directories, and deliberately not resolved beside
/// them: it belongs to the desktop, what goes in it is a registration with the
/// desktop rather than anything of Verkstead's to keep, and no variable of the
/// server's has anything to say about where it is.
export function autostartDir({ env }: Machine): string | undefined {
  const said = absolute(env.XDG_CONFIG_HOME);
  if (said !== undefined) {
    return join(said, AUTOSTART);
  }

  const home = absolute(env.HOME);
  return home === undefined ? undefined : join(home, ".config", AUTOSTART);
}

/// What the entry names: `$APPIMAGE` where it says an absolute path, and the
/// running executable otherwise.
///
/// An AppImage runs out of a filesystem its runtime mounted for this run alone,
/// so the executable there is a path under `/tmp` that will not be anything at
/// the next login — and the runtime sets that variable to say where the file the
/// human actually has is. There is no AppImage to meet until stage 05, and the
/// reading belongs beside the writing rather than in the stage that packs one.
///
/// Asked as a leading `/` rather than as `isAbsolute`, for the reason
/// [`absolute`](./platform.js) is: this is a Linux path, and the question has to
/// keep its answer wherever the arm is run.
export function named({ env, exe }: Registering): string {
  return absolute(env.APPIMAGE) ?? exe;
}

/// The entry as it is written, for the app at `exe`.
///
/// **[`HIDDEN`] is in it**, which is the one decision the entry makes: a login
/// start is an ordinary launch of this app in every other way, and a window
/// arriving over whatever the human is doing at every login is the thing that
/// gets the box unchecked. There is no verb in front of it — the tray app's
/// entry named one because its executable had several ways in, and this app is
/// the one way into itself.
///
/// The icon is named rather than pointed at, the way the specification means:
/// what a desktop draws beside this entry is whatever it has installed under the
/// app id, and a desktop that has none draws none.
export function written(exe: string): string {
  return (
    "[Desktop Entry]\n" +
    "Type=Application\n" +
    `Name=${NAME}\n` +
    `Comment=${COMMENT}\n` +
    `Exec=${quoted(exe)} ${HIDDEN}\n` +
    `Icon=${APP_ID}\n` +
    "Terminal=false\n"
  );
}

/// `exe` as an argument of an `Exec` line.
///
/// Quoted always rather than where it happens to need it: the specification
/// allows an argument to be quoted whole, and one rule is worth more here than a
/// table of which characters would have forced it. What is escaped inside the
/// quotes is what the specification says must be — the backslash for the value
/// itself, and then the quote, the backslash, the dollar and the backtick for
/// the argument inside it.
export function quoted(exe: string): string {
  let quoted = '"';

  for (const character of exe) {
    switch (character) {
      // Escaped for the argument and then escaped again for the value the
      // argument is written in, which is what makes four of one.
      case "\\":
        quoted += "\\\\\\\\";
        break;
      case '"':
      case "$":
      case "`":
        quoted += `\\\\${character}`;
        break;
      default:
        quoted += character;
    }
  }

  return `${quoted}"`;
}

/// Whether `entry` — a `.desktop` file as it was read — says Verkstead starts
/// with the session.
///
/// Being there is most of what an entry says, and two keys can say otherwise.
/// Both are how a desktop's own settings turn an entry off rather than delete
/// it: `Hidden`, which the specification defines as an entry the user has
/// deleted, and the key GNOME's own Startup Applications writes. Turning it off
/// *is* unchecking the box, so both are read and neither is argued with — and
/// both case-insensitively, nothing saying which case a desktop writes them in.
export function saysOn(entry: string): boolean {
  return !entry.split("\n").some((line) => {
    const at = line.indexOf("=");
    if (at < 0) {
      return false;
    }

    const key = line.slice(0, at).trim().toLowerCase();
    const value = line.slice(at + 1).trim().toLowerCase();

    return (
      (key === "hidden" && value === "true") ||
      (key === "x-gnome-autostart-enabled" && value === "false")
    );
  });
}

/// Whether this launch comes up with no window on the screen.
///
/// **A login start while the tray is shown, and nothing else** (ADR-0020, Set
/// 846 Q9). The icon is what makes a hidden app reachable, so a login start on a
/// machine with the tray off opens its window like any other launch. Which is
/// what keeps this a **Launch on Startup** rather than one that needs the tray.
///
/// **And a login start is either of the two ways of saying so.** [`HIDDEN`] in
/// the arguments is Linux's and Windows', whose registrations are command lines;
/// `atLogin` is the Mac's, whose login item carries none — see
/// [`Startup.atLogin`]. A launch by hand is neither, and is a window whatever
/// the tray says: somebody who started Verkstead is asking for it.
export function hidden(
  argv: readonly string[],
  chosen: Settings,
  atLogin: boolean,
): boolean {
  return (atLogin || argv.includes(HIDDEN)) && chosen.trayIcon;
}

/// The entry as it stands, or `undefined` where there is nothing to read.
function read(file: string): string | undefined {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return undefined;
  }
}

/// Write the entry, making the autostart directory where it is not there: a
/// machine that has never registered anything has never needed one, and the
/// specification's answer is to make it.
function write(file: string, entry: string): void {
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, entry, "utf8");
}

/// Take the entry away. One that is already gone is nothing to do rather than
/// anything to report: what was asked for is that Verkstead not start with the
/// session, and it does not.
function remove(file: string): void {
  rmSync(file, { force: true });
}
