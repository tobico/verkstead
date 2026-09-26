//! **The tray app's launch agent, taken over once** — the one thing about
//! **Launch on Startup** on a Mac that the login-item API knows nothing about.
//!
//! The Rust tray app registered itself by hand, with a property list at
//! `~/Library/LaunchAgents/net.tobico.Verkstead.plist` that `launchd` reads when
//! the user's agent domain comes up — see
//! `crates/desktop/src/startup/launchd.rs`, which is the file this reads and the
//! reading it makes of it. This app registers through Electron's login-item API
//! instead (ADR-0020, Set 846 Q9a), and the two know nothing of each other: left
//! alone after an upgrade, that agent is a launch at every login of a binary
//! this roadmap deletes, while the box on the Desktop page reads off.
//!
//! So the first launch after an upgrade **reads it, carries what it said into
//! the new registration, and removes it**. Once, because the file is then gone
//! and the next launch finds nothing.
//!
//! **What is read is presence plus two keys**, exactly as that module's
//! `says_on` reads them: `Disabled` set true is off, `RunAtLoad` set false is
//! off, and anything else — a key that is not there, a value that is not a
//! boolean, a file that is not really a plist — is the file being there, which
//! is most of what it says.
//!
//! **What the file *names* is not read at all.** It names the tray app's own
//! binary, the verb `desktop` and `--no-open`, and every one of those is a thing
//! this roadmap deletes: whichever it said, what stands afterwards is this app
//! registered through the API, or nothing.
//!
//! **On a Mac, and only where there is a registration to make.**
//! [`where`](./startup.js) answers `nowhere` on an unpackaged run by decision,
//! and a checkout run that deleted a developer's plist while offering them no
//! box to tick would be the worst of both.
//!
//! **And nothing is ever turned off behind anybody's back.** An agent that said
//! on becomes a login item; one that said off becomes nothing at all, rather
//! than an unregistration — a registration this app never made is not one it
//! takes away. The file goes in both cases.
//!
//! All of it is path, text and the injected [`LoginItem`](./startup.js), so
//! every arm is an ordinary unit test on this Linux runner — the agents
//! directory is read out of the values [`Machine`](./platform.js) carries, the
//! way the autostart directory is.

import { readFileSync, rmSync } from "node:fs";
import { join } from "node:path";

import { say } from "./log.js";
import { absolute, type Machine } from "./platform.js";
import { APP_ID, ARGS, where, type LoginItem, type Registering } from "./startup.js";

/// Where a user's own launch agents go, under the home directory.
const LAUNCH_AGENTS = "Library/LaunchAgents";

/// Where a Mac keeps this user's launch agents, or `undefined` where it names
/// nowhere to keep one.
///
/// Apple names this directory rather than leaving it to a variable, so the home
/// directory is the whole of the question — and absolute is asked of it as
/// [`absolute`](./platform.js) asks it, the same rule the autostart directory is
/// read by: a relative home is no answer, because a path resolved against
/// wherever the app was started from is nobody's launch agents.
export function agentsDir({ env }: Machine): string | undefined {
  const home = absolute(env.HOME);

  return home === undefined ? undefined : join(home, LAUNCH_AGENTS);
}

/// The tray app's own agent on this machine, or `undefined` where there is
/// nowhere for one.
///
/// Named for the app id, as that app named it and as the Linux entry is: the
/// file this reads is that app's file and never anybody else's.
export function agentFile(machine: Machine): string | undefined {
  const dir = agentsDir(machine);

  return dir === undefined ? undefined : join(dir, `${APP_ID}.plist`);
}

/// Whether `plist` — the tray app's launch agent as it was read — says Verkstead
/// starts at login.
///
/// `crates/desktop/src/startup/launchd.rs`'s `says_on`, key for key: being there
/// is most of what an agent says, and two keys can say otherwise. `Disabled` is
/// `launchd`'s own way of keeping an agent that is not to be run, and
/// `RunAtLoad` set false is an agent that is loaded and waits for something else
/// to start it — neither of which is Verkstead coming up with the login.
export function saysOn(plist: string): boolean {
  return flag(plist, "Disabled") !== true && flag(plist, "RunAtLoad") !== false;
}

/// Take the tray app's launch agent over, where this launch is one that can.
///
/// The whole of the upgrade: what the agent said becomes a registration through
/// the API or becomes nothing, and the agent goes either way. Called at a
/// launch, before the registration is rewritten — a plist that said on is a
/// registration this launch is to have made, and the rewrite is about one that
/// is already there.
///
/// **Nothing here fails.** A take-over that could not be made leaves the agent
/// exactly where it was and says so in the log, and the next launch tries again:
/// a Verkstead that would not serve because a file from the version before it
/// could not be deleted would be the worse of the two.
export function takeOver(registering: Registering, login: LoginItem): void {
  // The registration this would carry the agent into has to be one this run can
  // make: a Mac, and a packed app rather than a checkout.
  if (registering.platform !== "darwin" || !("login" in where(registering))) {
    return;
  }

  const file = agentFile(registering);
  if (file === undefined) {
    return;
  }

  // An agent nothing here can read is one nothing here can take over — and a
  // machine that never had the tray app is every machine from now on, which is
  // why this says nothing about finding none.
  const plist = read(file);
  if (plist === undefined) {
    return;
  }

  const on = saysOn(plist);
  say(
    `the tray app's launch agent at ${file} says Verkstead ` +
      `${on ? "starts" : "does not start"} at login, and this launch takes it over`,
  );

  try {
    // Only where it said on: an agent that said off becomes nothing rather than
    // an unregistration, this app never having made the registration it would
    // be taking away.
    if (on) {
      login.register({ openAtLogin: true, args: ARGS });
    }

    // After the registration rather than before it, so that a take-over that
    // was refused leaves the agent to be read again — what it says is the only
    // record of what the human asked for.
    rmSync(file, { force: true });
  } catch (trouble) {
    say(
      `taking over the tray app's launch agent at ${file} did not come off — ` +
        `${String(trouble)}; it is left where it is and the next launch tries again`,
    );
  }
}

/// What `key` is set to in `plist`, where it is set to a boolean at all.
///
/// A reader for the two keys above rather than a plist parser, and the Rust
/// arm's own: what is being asked of the file is whether somebody turned it off
/// since it was written, and anything else in it is this having nothing to say.
function flag(plist: string, key: string): boolean | undefined {
  const after = plist.split(`<key>${key}</key>`)[1]?.trimStart();
  if (after === undefined) {
    return undefined;
  }

  if (after.startsWith("<true")) {
    return true;
  }

  return after.startsWith("<false") ? false : undefined;
}

/// The agent as it stands, or `undefined` where there is nothing to read.
function read(file: string): string | undefined {
  try {
    return readFileSync(file, "utf8");
  } catch {
    return undefined;
  }
}
