//! **The tray app's launch agent, taken over once**: what is read out of the
//! plist, what becomes of it, and the launches that leave it alone.
//!
//! `crates/desktop/src/startup/launchd.rs`'s own suite for the reading half,
//! arm for arm, because the file being read is the file that app wrote: the
//! directory under the home, the name the app id gives it, and the two keys
//! `says_on` turns an agent off by.
//!
//! **The failure this is here to catch is a launch agent left behind.** After
//! an upgrade that plist still starts a binary this roadmap deletes at every
//! login, while the box on the Desktop page reads off — a Verkstead that says
//! it does not start with the session and a login that keeps trying to start
//! one. So what the agent said has to arrive in the new registration, the file
//! has to go, and the machines that never had it — and the runs that have no
//! registration to make — have to be left exactly as they are.
//!
//! All of it is path, text and the injected login-item API, so every arm runs
//! here on Linux: the home directory is a value handed in, as the autostart
//! directory's is.

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { beforeEach, describe, expect, it } from "vitest";

import { agentFile, agentsDir, saysOn, takeOver } from "../src/launchd.js";
import {
  APP_ID,
  ARGS,
  type LoginAsked,
  type LoginItem,
  type Registering,
} from "../src/startup.js";

/// The app as an upgraded Mac runs it: a packed Verkstead whose home directory
/// is `home`.
const packed = (home: string, over: Partial<Registering> = {}): Registering => ({
  packaged: true,
  platform: "darwin",
  env: { HOME: home },
  exe: "/Applications/Verkstead.app/Contents/MacOS/Verkstead",
  ...over,
});

/// The agent as the Rust tray app wrote it, for a Verkstead at `exe` — the
/// text `crates/desktop/src/startup/launchd.rs` writes, which is what a machine
/// being upgraded actually has on it.
const wrote = (exe = "/usr/local/bin/verkstead"): string =>
  '<?xml version="1.0" encoding="UTF-8"?>\n' +
  '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" ' +
  '"http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n' +
  '<plist version="1.0">\n<dict>\n' +
  `\t<key>Label</key>\n\t<string>${APP_ID}</string>\n` +
  "\t<key>ProgramArguments</key>\n\t<array>\n" +
  `\t\t<string>${exe}</string>\n\t\t<string>desktop</string>\n` +
  "\t\t<string>--no-open</string>\n\t</array>\n" +
  "\t<key>RunAtLoad</key>\n\t<true/>\n" +
  "</dict>\n</plist>\n";

/// Electron's login-item API, stood in for: what it was asked, and nothing
/// else — the take-over never reads one back.
function loginItem(): LoginItem & { asked: LoginAsked[] } {
  const asked: LoginAsked[] = [];

  return {
    asked,
    registered: () => false,
    openedAtLogin: () => false,
    status: () => undefined,
    register: (wanted) => asked.push(wanted),
  };
}

let home: string;

/// Where the agent is under the home directory this test made, and the writing
/// of one there.
const agentIn = (at: string): string => join(at, "Library/LaunchAgents", `${APP_ID}.plist`);

const leave = (plist: string, at = home): string => {
  const file = agentIn(at);

  mkdirSync(join(at, "Library/LaunchAgents"), { recursive: true });
  writeFileSync(file, plist, "utf8");

  return file;
};

beforeEach(() => {
  home = mkdtempSync(join(tmpdir(), "verkstead-launchd-"));
});

describe("where the tray app's agent is", () => {
  /// Apple's own directory, which is what every Mac has.
  it("is under the home directory", () => {
    expect(agentsDir({ platform: "darwin", env: { HOME: "/Users/you" } })).toBe(
      "/Users/you/Library/LaunchAgents",
    );
  });

  /// The rule the autostart directory is read by, and the one
  /// `verkstead_server::platform` reads a home directory by: a relative path is
  /// no answer, and a machine that says nothing about where its home is has
  /// nowhere to have kept an agent.
  it("is nowhere without an absolute home", () => {
    expect(agentsDir({ platform: "darwin", env: { HOME: "Users/you" } })).toBeUndefined();
    expect(agentsDir({ platform: "darwin", env: {} })).toBeUndefined();
  });

  /// And the file is that app's own and says so in its name, as the label
  /// inside it says the same thing.
  it("is the app id's own plist", () => {
    expect(agentFile({ platform: "darwin", env: { HOME: "/Users/you" } })).toBe(
      "/Users/you/Library/LaunchAgents/net.tobico.Verkstead.plist",
    );
    expect(agentFile({ platform: "darwin", env: {} })).toBeUndefined();
  });
});

describe("what an agent is read as", () => {
  /// The whole of the state: the file the tray app wrote is there, so Verkstead
  /// started at login.
  it("is on where it is simply there", () => {
    expect(saysOn(wrote())).toBe(true);
    expect(saysOn("")).toBe(true);
  });

  /// And the two keys that say otherwise, which are `launchd`'s own way of
  /// keeping an agent that is not to be run and an agent that waits to be
  /// started by something else.
  it("is off where Disabled says true or RunAtLoad says false", () => {
    expect(saysOn(`${wrote()}<key>Disabled</key>\n<true/>\n`)).toBe(false);
    expect(saysOn(`${wrote()}<key>Disabled</key>\n<false/>\n`)).toBe(true);
    expect(
      saysOn(
        '<plist version="1.0">\n<dict>\n\t<key>RunAtLoad</key>\n\t<false/>\n</dict>\n</plist>',
      ),
    ).toBe(false);
  });

  /// Anything else in the file is this having nothing to say, which is the file
  /// being there — the Rust arm's own reading, and the one a hand-edited plist
  /// has to be given.
  it("is on where neither key is set to a boolean", () => {
    expect(saysOn("<key>Disabled</key>\n<string>true</string>\n")).toBe(true);
    expect(saysOn("<key>RunAtLoad</key>")).toBe(true);
    expect(saysOn("not a plist at all")).toBe(true);
  });
});

describe("the take-over", () => {
  /// The upgrade this is all for: the machine started Verkstead at login under
  /// the tray app, and it goes on doing it under this one — through the API,
  /// with the plist gone.
  it("carries an agent that said on into a registration, and removes it", () => {
    const file = leave(wrote());
    const login = loginItem();

    takeOver(packed(home), login);

    expect(login.asked).toEqual([{ openAtLogin: true, args: ARGS }]);
    expect(existsSync(file)).toBe(false);
  });

  /// And one that said off becomes nothing rather than an unregistration: this
  /// app never made the registration it would be taking away, and an agent
  /// naming a binary that is gone is worth nothing either way.
  it("carries an agent that said off into nothing, and removes it too", () => {
    for (const off of ["Disabled</key>\n<true/>", "RunAtLoad</key>\n<false/>"]) {
      const file = leave(`<plist version="1.0">\n<dict>\n<key>${off}\n</dict>\n</plist>\n`);
      const login = loginItem();

      takeOver(packed(home), login);

      expect(login.asked, off).toEqual([]);
      expect(existsSync(file), off).toBe(false);
    }
  });

  /// Once, and the file being gone is the whole of what makes it once: the
  /// launch after the take-over finds nothing and asks for nothing.
  it("is nothing the next launch does", () => {
    leave(wrote());
    takeOver(packed(home), loginItem());

    const login = loginItem();
    takeOver(packed(home), login);

    expect(login.asked).toEqual([]);
  });

  /// A machine that never had the tray app is every machine from here on: there
  /// is nothing to read, nothing is registered, and the directory is not so
  /// much as made.
  it("is nothing on a machine carrying no agent", () => {
    const login = loginItem();

    takeOver(packed(home), login);

    expect(login.asked).toEqual([]);
    expect(readdirSync(home)).toEqual([]);
  });

  /// The run with nothing worth registering: a checkout run that deleted a
  /// developer's plist while offering them no box to tick would be the worst of
  /// both.
  it("is nothing an unpackaged run does", () => {
    const file = leave(wrote());
    const login = loginItem();

    takeOver(packed(home, { packaged: false }), login);

    expect(login.asked).toEqual([]);
    expect(readFileSync(file, "utf8")).toBe(wrote());
  });

  /// And it is a Mac's alone. Neither of the other two has a launch agent to
  /// find, and a file under `Library/LaunchAgents` on one of them is somebody
  /// else's business whatever it says.
  it("is nothing the other two platforms do", () => {
    for (const platform of ["linux", "win32"] as const) {
      const file = leave(wrote());
      const login = loginItem();

      takeOver(packed(home, { platform }), login);

      expect(login.asked, platform).toEqual([]);
      expect(readFileSync(file, "utf8"), platform).toBe(wrote());
    }
  });

  /// A machine that names nowhere to have kept one is asked nothing about it.
  it("is nothing where the machine names no home", () => {
    const login = loginItem();

    takeOver(packed(home, { env: {} }), login);

    expect(login.asked).toEqual([]);
  });

  /// And a take-over the API refused leaves the agent where it is: what it says
  /// is the only record of what the human asked for, so the next launch is to
  /// read it again rather than find it gone.
  it("leaves the agent behind where the registration could not be made", () => {
    const file = leave(wrote());
    const login: LoginItem = {
      ...loginItem(),
      register: () => {
        throw new Error("the login item could not be written");
      },
    };

    takeOver(packed(home), login);

    expect(readFileSync(file, "utf8")).toBe(wrote());
  });
});
