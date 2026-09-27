//! **The tray app's Run value, taken over once** — the one thing about **Launch
//! on Startup** on Windows that the login-item API knows nothing about.
//!
//! The Rust tray app registered itself with a value under
//! `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`, named for
//! the app id and holding `"<exe>" desktop --no-open` — see
//! `crates/desktop/src/startup/run_key.rs`, which is the value this reads and the
//! reading it makes of it. This app registers through Electron's login-item API
//! instead (ADR-0020, Set 846 Q9a), which writes a value of its own under
//! [`VALUE`](./startup.js) and has never heard of that one: left alone after an
//! upgrade, it is a sign-in start at every sign-in of a binary this roadmap
//! deletes, while the box on the Desktop page reads off.
//!
//! So the first launch after an upgrade **reads it, carries what it said into the
//! new registration, and removes it**. Once, because the value is then gone and
//! the next launch finds nothing.
//!
//! **What is read is presence, and that is the whole of it.** A registry value
//! has no `Disabled` and no `X-GNOME-Autostart-enabled`: the value is there and
//! Verkstead started at sign-in, or it is not and Verkstead did not, which is
//! what that module says for itself. So this is the shorter of the two
//! take-overs — there is no key in it that could have said otherwise, and no
//! branch for one that did.
//!
//! **What it *names* is not read at all**, the way the plist's command is not. It
//! names the tray app's own binary, the verb `desktop` and `--no-open`, and every
//! one of those is a thing this roadmap deletes: whichever it said, what stands
//! afterwards is this app registered through the API.
//!
//! **And what this cannot see is the Startup tab.** A human who switched
//! Verkstead off in Task Manager is recorded in a key of Explorer's own —
//! `StartupApproved` — which is nothing to do with this value, so a value that is
//! there is read as on either way. That is the honest answer
//! `crates/desktop/src/startup/run_key.rs` already gave about the value it wrote,
//! and it carries over unchanged. It is *not* the answer about the registration
//! that replaces it: Electron reads Explorer's key back for a value of its own,
//! and [`switchedOff`](./startup.js) is where that is read.
//!
//! **On Windows, and only where there is a registration to make.**
//! [`where`](./startup.js) answers `nowhere` on an unpackaged run by decision,
//! and a checkout run that deleted a developer's Run value while offering them no
//! box to tick would be the worst of both.
//!
//! **Reading and deleting the value is [`registry.ts`](./registry.js)'s**, that
//! being the one thing this app has no API for — `reg.exe` run and its exit
//! status read, injected as [`Registry`](./registry.js) the way the login-item
//! API is, which is what makes every arm an ordinary vitest on the Linux runner.
//! A value that is not there, a key that is not there and a command that would
//! not run are one answer there: nothing here to take over.
//!
//! **And nothing here fails.** A take-over that could not be made leaves the
//! value where it was and says so in the log, and the next launch tries again: a
//! Verkstead that would not serve because a value from the version before it
//! could not be deleted would be the worse of the two — exactly as the Mac arm
//! has it.

import { say } from "./log.js";
import { reg, type Registry, RUN } from "./registry.js";
import { APP_ID, ARGS, VALUE, where, type LoginItem, type Registering } from "./startup.js";

/// Take the tray app's Run value over, where this launch is one that can.
///
/// The whole of the upgrade: the value being there becomes a registration through
/// the API, and the value goes. Called at a launch, before the registration is
/// rewritten — a value that was there is a registration this launch is to have
/// made, and the rewrite is about one that is already there.
///
/// [`takeOver`](./launchd.js)'s twin, down to where it is called from in
/// `main.ts`, and shorter by the two keys a plist can be turned off with.
export function takeOver(
  registering: Registering,
  login: LoginItem,
  registry: Registry = reg,
): void {
  // The registration this would carry the value into has to be one this run can
  // make: Windows, and a packed app rather than a checkout.
  if (registering.platform !== "win32" || !("login" in where(registering))) {
    return;
  }

  // A value nothing here can find is one nothing here can take over — and a
  // machine that never had the tray app is every machine from now on, which is
  // why this says nothing about finding none.
  if (!registry.has(RUN, APP_ID)) {
    return;
  }

  say(
    `the tray app's Run value ${APP_ID} is there, so Verkstead started at ` +
      `sign-in and this launch takes that over`,
  );

  try {
    login.register({ openAtLogin: true, args: ARGS, name: VALUE });

    // After the registration rather than before it, so that a take-over that was
    // refused leaves the value to be read again — it is the only record of what
    // the human asked for.
    registry.remove(RUN, APP_ID);
  } catch (trouble) {
    say(
      `taking over the tray app's Run value ${APP_ID} did not come off — ` +
        `${String(trouble)}; it is left where it is and the next launch tries again`,
    );
  }
}
