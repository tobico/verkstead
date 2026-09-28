//! **The tray app's Run value, taken over once**: what is found under the Run
//! key, what becomes of it, and the launches that leave it alone.
//!
//! The Rust tray app's own reading, which is the whole of
//! what that arm ever asked of a value: it is there and Verkstead started at
//! sign-in, or it is not and Verkstead did not. So this suite is `launchd.test.ts`
//! without the two keys a plist can be turned off with, and with one arm that
//! file has no need of — the value this app writes not being the value this reads.
//!
//! **The failure this is here to catch is a sign-in start left behind.** After an
//! upgrade that value still starts a binary that is gone at every sign-in,
//! while the box on the Desktop page reads off — a Verkstead that says it does not
//! start with the session and a sign-in that keeps trying to start one. So the
//! value has to arrive in the new registration, it has to go, and the machines
//! that never had it — and the runs with no registration to make — have to be left
//! exactly as they are.
//!
//! **And the other failure is a box that unregisters itself.** Electron names the
//! value it writes after the AppUserModelId where it is told nothing else, and
//! that id is the app id — the name this reads. One name for both and the launch
//! after a take-over reads this app's own registration as the tray app's,
//! registers again and deletes it, every other launch. Nothing but the two names
//! being different protects that, so it is asserted here as well as in
//! `startup.test.ts`.
//!
//! `reg.exe` is injected as [`Registry`](../src/runkey.js) the way the login-item
//! API is, so every arm runs here on Linux: a registry a test can put a value in
//! and read the calls back off.

import { describe, expect, it } from "vitest";

import { type Registry, RUN } from "../src/registry.js";
import { takeOver } from "../src/runkey.js";
import {
  APP_ID,
  ARGS,
  type LoginAsked,
  type LoginItem,
  type Registering,
  VALUE,
} from "../src/startup.js";

/// The app as an upgraded Windows machine runs it: a packed Verkstead installed
/// where the msi puts one.
const packed = (over: Partial<Registering> = {}): Registering => ({
  packaged: true,
  platform: "win32",
  env: {},
  exe: String.raw`C:\Users\you\AppData\Local\Programs\Verkstead\Verkstead.exe`,
  ...over,
});

/// Electron's login-item API, stood in for: what it was asked, and nothing else —
/// the take-over never reads one back.
function loginItem(): LoginItem & { asked: LoginAsked[] } {
  const asked: LoginAsked[] = [];

  return {
    asked,
    registered: () => false,
    values: () => [],
    openedAtLogin: () => false,
    status: () => undefined,
    register: (wanted) => asked.push(wanted),
  };
}

/// The registry, stood in for: the values that are there, and what was asked of
/// it.
///
/// `has` and `remove` both record, because *not* asking is half of what several
/// of these arms are about: a launch off this platform reads nothing at all, and
/// a take-over that could not register must not have deleted anything.
function registry(...values: string[]): Registry & { asked: string[]; held: string[] } {
  const asked: string[] = [];
  const held = [...values];

  return {
    asked,
    held,
    has: (key, value) => {
      asked.push(`has ${key} ${value}`);
      return held.includes(`${key} ${value}`);
    },
    remove: (key, value) => {
      asked.push(`remove ${key} ${value}`);

      const at = held.indexOf(`${key} ${value}`);
      if (at < 0) {
        return;
      }
      held.splice(at, 1);
    },
  };
}

/// The tray app's own value, as a machine being upgraded has it.
const wrote = `${RUN} ${APP_ID}`;

describe("where the tray app's value is", () => {
  /// The current user's own key, which is the one the tray app wrote and the one
  /// Electron's API writes — `HKEY_LOCAL_MACHINE` would be every account on the
  /// machine, and writing it wants an elevation nobody asked for.
  it("is under the current user's Run key", () => {
    expect(RUN).toBe(String.raw`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`);
  });

  /// And it is that app's own and says so in its name, which is the app id —
  /// the Rust tray app named its value for the same string.
  it("is the app id's own value", () => {
    expect(APP_ID).toBe("net.tobico.Verkstead");
  });
});

describe("what the take-over reads", () => {
  /// One value under one key, and nothing else asked of the registry: the value
  /// being there is the whole of the state, a registry value having no
  /// `Disabled` and no `X-GNOME-Autostart-enabled` to say otherwise.
  it("is that one value and nothing beside it", () => {
    const keys = registry();

    takeOver(packed(), loginItem(), keys);

    expect(keys.asked).toEqual([`has ${RUN} ${APP_ID}`]);
  });
});

describe("the take-over", () => {
  /// The upgrade this is all for: the machine started Verkstead at sign-in under
  /// the tray app, and it goes on doing it under this one — through the API, under
  /// a name of its own, with the old value gone.
  it("carries a value that is there into a registration, and removes it", () => {
    const keys = registry(wrote);
    const login = loginItem();

    takeOver(packed(), login, keys);

    expect(login.asked).toEqual([{ openAtLogin: true, args: ARGS, name: VALUE }]);
    expect(keys.held).toEqual([]);
  });

  /// Once, and the value being gone is the whole of what makes it once: the
  /// launch after the take-over finds nothing and asks for nothing.
  it("is nothing the next launch does", () => {
    const keys = registry(wrote);
    takeOver(packed(), loginItem(), keys);

    const login = loginItem();
    takeOver(packed(), login, keys);

    expect(login.asked).toEqual([]);
  });

  /// **And the registration it just made is not what the next launch reads.** The
  /// value this app writes is called something else on purpose — see
  /// `startup.ts` — so a take-over followed by a launch leaves the box ticked
  /// rather than unregistering it.
  it("does not read the value the API writes", () => {
    const keys = registry(wrote);
    takeOver(packed(), loginItem(), keys);

    // What the API's own write left behind, under its own name.
    keys.held.push(`${RUN} ${VALUE}`);

    const login = loginItem();
    takeOver(packed(), login, keys);

    expect(login.asked).toEqual([]);
    expect(keys.held).toEqual([`${RUN} ${VALUE}`]);
  });

  /// A machine that never had the tray app is every machine from here on: there
  /// is nothing to find, nothing is registered, and nothing is deleted.
  it("is nothing on a machine carrying no value", () => {
    const keys = registry();
    const login = loginItem();

    takeOver(packed(), login, keys);

    expect(login.asked).toEqual([]);
    expect(keys.asked).toEqual([`has ${RUN} ${APP_ID}`]);
  });

  /// The run with nothing worth registering: a checkout run that deleted a
  /// developer's Run value while offering them no box to tick would be the worst
  /// of both. Nothing is so much as read.
  it("is nothing an unpackaged run does", () => {
    const keys = registry(wrote);
    const login = loginItem();

    takeOver(packed({ packaged: false }), login, keys);

    expect(login.asked).toEqual([]);
    expect(keys.asked).toEqual([]);
    expect(keys.held).toEqual([wrote]);
  });

  /// And it is Windows' alone. Neither of the other two has a Run key to read at
  /// all, and the registry is not asked about one.
  it("is nothing the other two platforms do", () => {
    for (const platform of ["linux", "darwin"] as const) {
      const keys = registry(wrote);
      const login = loginItem();

      takeOver(packed({ platform }), login, keys);

      expect(login.asked, platform).toEqual([]);
      expect(keys.asked, platform).toEqual([]);
      expect(keys.held, platform).toEqual([wrote]);
    }
  });

  /// And a take-over the API refused leaves the value where it is: it is the only
  /// record of what the human asked for, so the next launch is to read it again
  /// rather than find it gone.
  it("leaves the value behind where the registration could not be made", () => {
    const keys = registry(wrote);
    const login: LoginItem = {
      ...loginItem(),
      register: () => {
        throw new Error("the login item could not be written");
      },
    };

    takeOver(packed(), login, keys);

    expect(keys.held).toEqual([wrote]);
    expect(keys.asked).toEqual([`has ${RUN} ${APP_ID}`]);
  });

  /// A delete that would not go through is the same: said in the log, the value
  /// left where it was, and the launch after this one reads it again. The
  /// registration stands, which is the half of it that came off — a Verkstead
  /// that would not serve because a value from the version before it could not be
  /// deleted would be the worse of the two.
  it("leaves the value behind where it could not be deleted", () => {
    const keys: Registry = {
      has: () => true,
      remove: () => {
        throw new Error("the value could not be deleted");
      },
    };
    const login = loginItem();

    takeOver(packed(), login, keys);

    expect(login.asked).toEqual([{ openAtLogin: true, args: ARGS, name: VALUE }]);
  });
});

/// `reg.exe` itself is `registry.test.ts`'s.
