//! **Launch on Startup**: where the registration goes, what it says, and what
//! reading it back comes to.
//!
//! `crates/desktop/src/startup/xdg.rs`'s own suite, arm for arm, because the
//! entry this app writes is the entry that app wrote — the same file under the
//! same name, which is how the old registration is taken over rather than
//! doubled (ADR-0020). So the directory resolution, the quoting and the two
//! off-readings are asked here exactly as they are asked there, and the one
//! thing that has changed is the command: no verb, and a flag of this app's own
//! saying come up hidden.
//!
//! **The failure this is here to catch is a registration nothing will honour.**
//! An entry naming a path a login cannot run is a box that ticks and does
//! nothing at all — which is the one failure a human never sees until the
//! morning after, when Verkstead did not come up. So the text is pinned rather
//! than described, and the reading of an entry a desktop's own settings turned
//! off is pinned with it: a machine where somebody unticked Verkstead in GNOME's
//! Startup Applications has to read as unticked here.
//!
//! The Mac and Windows arms are Electron's login-item API, which is a call
//! rather than a file — so they are exercised through a stub of the two calls
//! this needs of it, which is what makes the arm this Linux runner will never
//! run an ordinary test all the same.

import { existsSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Settings } from "../src/settings.js";
import {
  APP_ID,
  autostartDir,
  HIDDEN,
  hidden,
  named,
  quoted,
  type LoginAsked,
  type LoginItem,
  type Registering,
  saysOn,
  startup,
  where,
  written,
} from "../src/startup.js";

/// The app as a login would have to start it: a packed Verkstead on a Linux box
/// whose configuration directory is `dir`.
const packed = (dir: string, over: Partial<Registering> = {}): Registering => ({
  packaged: true,
  platform: "linux",
  env: { XDG_CONFIG_HOME: dir },
  exe: "/opt/verkstead/verkstead",
  ...over,
});

/// Where the entry goes under a configuration directory, which is the one path
/// every test about the file reads.
const entryIn = (dir: string): string => join(dir, "autostart", `${APP_ID}.desktop`);

/// Electron's login-item API, stood in for: what it was asked, and whatever it
/// is to answer next.
function loginItem(on = false): LoginItem & { asked: LoginAsked[] } {
  const asked: LoginAsked[] = [];
  let registered = on;

  return {
    asked,
    registered: () => registered,
    register: (wanted) => {
      asked.push(wanted);
      registered = wanted.openAtLogin;
    },
  };
}

/// One that is never called, for the arms that are a file rather than a call.
const noLogin: LoginItem = {
  registered: () => {
    throw new Error("the Linux arm is a file, and asked the login-item API");
  },
  register: () => {
    throw new Error("the Linux arm is a file, and asked the login-item API");
  },
};

let dir: string;

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "verkstead-startup-"));
});

afterEach(() => vi.restoreAllMocks());

describe("where autostart entries go", () => {
  /// The specification's own directory, which is what nearly every machine has.
  it("is under the home directory when nothing says otherwise", () => {
    expect(autostartDir({ platform: "linux", env: { HOME: "/home/you" } })).toBe(
      "/home/you/.config/autostart",
    );
  });

  it("is where an absolute XDG_CONFIG_HOME says", () => {
    expect(
      autostartDir({
        platform: "linux",
        env: { XDG_CONFIG_HOME: "/etc/xdg-you", HOME: "/home/you" },
      }),
    ).toBe("/etc/xdg-you/autostart");
  });

  /// The specification's own rule about its variables, and the one
  /// `platform.ts` reads the Data Directory by: a relative path is no answer,
  /// because a directory resolved against wherever the app was started is the
  /// thing the default replaces.
  it("ignores a relative one, and answers nowhere with nothing absolute at all", () => {
    expect(
      autostartDir({ platform: "linux", env: { XDG_CONFIG_HOME: "xdg", HOME: "/home/you" } }),
    ).toBe("/home/you/.config/autostart");
    expect(
      autostartDir({ platform: "linux", env: { XDG_CONFIG_HOME: "xdg", HOME: "." } }),
    ).toBeUndefined();
    expect(autostartDir({ platform: "linux", env: {} })).toBeUndefined();
  });
});

describe("where the registration goes", () => {
  /// The file is Verkstead's own and says so in its name — the same name under
  /// the same directory the Rust tray app wrote, which is how that registration
  /// is taken over rather than doubled.
  it("is the app id's own entry on Linux", () => {
    expect(where(packed(dir))).toEqual({ entry: entryIn(dir) });
  });

  /// And Electron's own API on the other two, which is what ADR-0020 says: the
  /// app is always a bundle now, so there is no plist to hand-write.
  it("is the login-item API on a Mac and on Windows", () => {
    for (const platform of ["darwin", "win32"] as const) {
      expect(where(packed(dir, { platform })), platform).toEqual({ login: true });
    }
  });

  /// The one that is nowhere for a reason about the app rather than about the
  /// machine: every run in this stage is `pnpm start` from a checkout, and what
  /// an entry could name there is the dev shell's Electron plus a build
  /// directory — a registration that breaks the next time either moves.
  it("is nowhere at all on an unpackaged run, whatever the platform", () => {
    for (const platform of ["linux", "darwin", "win32"] as const) {
      const put = where(packed(dir, { platform, packaged: false }));

      expect("nowhere" in put, platform).toBe(true);
      expect("nowhere" in put && put.nowhere).toMatch(/installed/);
    }
  });

  /// And the machine's own nowhere: a Linux box that names no configuration
  /// directory has no autostart directory to put an entry in.
  it("is nowhere on a machine that names no configuration directory", () => {
    const put = where(packed(dir, { env: {} }));

    expect("nowhere" in put).toBe(true);
    expect("nowhere" in put && put.nowhere).toMatch(/nowhere/);
  });
});

describe("what the entry says", () => {
  /// What a login starts, and the one decision the entry makes about how: the
  /// app as anybody else starts it, with no window over whatever the human is
  /// doing.
  it("starts the app with the flag that says come up hidden", () => {
    const entry = written("/opt/verkstead/verkstead");

    expect(entry).toContain(`Exec="/opt/verkstead/verkstead" ${HIDDEN}\n`);
    expect(entry.startsWith("[Desktop Entry]\n")).toBe(true);
    expect(entry).toContain("Type=Application\n");
    expect(entry).toContain("Name=Verkstead\n");
    expect(entry).toContain(`Icon=${APP_ID}\n`);
    expect(entry).toContain("Terminal=false\n");
  });

  /// And what does *not* carry over from the entry the Rust app wrote: that one
  /// named a verb of the CLI it was a verb of, and `--no-open` for the browser
  /// it would otherwise have been handed. This app is its own way in and opens
  /// no browser at all.
  it("says no verb, this app being its own way in", () => {
    const entry = written("/opt/verkstead/verkstead");

    expect(entry).not.toContain("desktop");
    expect(entry).not.toContain("--no-open");
  });

  /// A path a desktop would otherwise read as two arguments, which is what the
  /// quoting is there for — a downloads directory with a space in it is exactly
  /// where an app somebody moved ends up.
  it("quotes the path whole, whatever is in it", () => {
    expect(quoted("/home/you/My Apps/verkstead")).toBe('"/home/you/My Apps/verkstead"');
    expect(quoted("/home/$you/it`s")).toBe('"/home/\\\\$you/it\\\\`s"');
    expect(quoted("/home/you/a\\b")).toBe('"/home/you/a\\\\\\\\b"');
  });
});

describe("what an entry is read as", () => {
  it("is a checked box where it is simply there", () => {
    expect(saysOn(written("/opt/verkstead/verkstead"))).toBe(true);
  });

  /// Both of the ways a desktop's own settings turn an entry off rather than
  /// delete it: the specification's `Hidden`, and the key GNOME's own Startup
  /// Applications writes. Turning it off *is* unchecking the box, so both are
  /// read and neither is argued with.
  it("is unchecked where a desktop turned it off", () => {
    expect(saysOn("[Desktop Entry]\nType=Application\nHidden=true\n")).toBe(false);
    expect(saysOn("[Desktop Entry]\nType=Application\nX-GNOME-Autostart-enabled=false\n")).toBe(
      false,
    );
    expect(saysOn("[Desktop Entry]\nType=Application\nX-GNOME-Autostart-enabled=true\n")).toBe(
      true,
    );
  });

  /// Read the way the Rust arm reads them, which is the way a file somebody
  /// hand-edited has to be read: the keys are a desktop's rather than this
  /// app's, and nothing says which case they were written in.
  it("reads both keys and both values whatever their case, and past the spaces", () => {
    expect(saysOn("[Desktop Entry]\nhidden = TRUE\n")).toBe(false);
    expect(saysOn("[Desktop Entry]\n  X-Gnome-Autostart-Enabled=False  \n")).toBe(false);
    expect(saysOn("[Desktop Entry]\nHiddenSomething=true\n")).toBe(true);
    expect(saysOn("[Desktop Entry]\nName=Hidden=true\n")).toBe(true);
  });

  /// And the entry the Rust tray app left behind is a checked box, because it is
  /// the same file: the first launch of this app on a machine that had the other
  /// one finds a registration and reads it as one.
  it("reads the entry the tray app wrote as a checked box", () => {
    expect(
      saysOn(
        "[Desktop Entry]\nType=Application\nName=Verkstead\n" +
          'Exec="/usr/local/bin/verkstead" desktop --no-open\nTerminal=false\n',
      ),
    ).toBe(true);
  });
});

describe("what the entry names", () => {
  /// An AppImage runs out of a filesystem its runtime mounted for this run
  /// alone, so the executable there is a path under `/tmp` that will not be
  /// anything at the next login — and the runtime sets the variable to say where
  /// the file the human actually has is.
  it("is the AppImage where the variable names an absolute one", () => {
    expect(
      named(packed(dir, { env: { APPIMAGE: "/home/you/Apps/Verkstead-x86_64.AppImage" } })),
    ).toBe("/home/you/Apps/Verkstead-x86_64.AppImage");
  });

  it("is the running executable otherwise", () => {
    for (const env of [{}, { APPIMAGE: "" }, { APPIMAGE: "relative/Verkstead.AppImage" }]) {
      expect(named(packed(dir, { env })), JSON.stringify(env)).toBe("/opt/verkstead/verkstead");
    }
  });
});

describe("the box", () => {
  it("writes the entry when it is checked and takes it away when it is not", () => {
    const starts = startup(packed(dir), noLogin);

    expect(starts.standing()).toEqual({ possible: true, on: false });

    expect(starts.set(true)).toEqual({ possible: true, on: true });
    expect(readFileSync(entryIn(dir), "utf8")).toContain(
      `Exec="/opt/verkstead/verkstead" ${HIDDEN}`,
    );

    expect(starts.set(false)).toEqual({ possible: true, on: false });
    expect(existsSync(entryIn(dir))).toBe(false);
  });

  /// The directory is made where it is not there: a machine that has never
  /// registered anything has never needed one, and the specification's answer is
  /// to make it.
  it("makes the autostart directory, and writes nothing else in it", () => {
    startup(packed(dir), noLogin).set(true);

    expect(existsSync(entryIn(dir))).toBe(true);
    expect(readdirSync(join(dir, "autostart"))).toEqual([`${APP_ID}.desktop`]);
  });

  /// The state is the registration's, so a registration taken away by somebody
  /// else — a desktop's Startup Applications, or `rm` — is a box that comes back
  /// unchecked rather than one Verkstead argues about.
  it("is unchecked where the entry a desktop turned off says so", () => {
    const starts = startup(packed(dir), noLogin);
    starts.set(true);

    writeFileSync(entryIn(dir), "[Desktop Entry]\nType=Application\nHidden=true\n", "utf8");

    expect(starts.standing()).toEqual({ possible: true, on: false });
  });

  /// Unchecking a box that is already unchecked is nothing to do rather than
  /// anything to report: what was asked for is that Verkstead not start with the
  /// session, and it does not.
  it("takes away an entry that is already gone without complaining", () => {
    expect(startup(packed(dir), noLogin).set(false)).toEqual({ possible: true, on: false });
  });

  /// A write that could not be made is a box that goes back where it was, with
  /// what the platform said carried up to the page: the registration is the
  /// state, so a set that wrote nothing has changed nothing.
  it("says what refused a registration it could not make", () => {
    const blocked = join(dir, "a-file-not-a-directory");
    writeFileSync(blocked, "", "utf8");

    const now = startup(packed(dir, { env: { XDG_CONFIG_HOME: blocked } }), noLogin).set(true);

    expect(now.possible).toBe(true);
    expect(now.on).toBe(false);
    expect(now.refused).toMatch(/could not/);
  });
});

describe("a machine with nowhere to keep one", () => {
  /// The box is greyed with the reason on it rather than one that ticks and does
  /// nothing — and a set arriving all the same writes nothing.
  it("is a box that cannot be ticked, and says why", () => {
    const starts = startup(packed(dir, { packaged: false }), noLogin);
    const standing = starts.standing();

    expect(standing.possible).toBe(false);
    expect(standing.on).toBe(false);
    expect(standing.why).toMatch(/installed/);

    expect(starts.set(true)).toEqual(standing);
    expect(existsSync(entryIn(dir))).toBe(false);
  });

  /// And a launch of one registers nothing at all, which is the rule every arm
  /// holds: what a launch rewrites is what somebody asked for.
  it("registers nothing at a launch", () => {
    startup(packed(dir, { packaged: false }), noLogin).refresh();

    expect(existsSync(entryIn(dir))).toBe(false);
  });
});

describe("a launch", () => {
  /// The launch of an app that has moved: the registration it left behind names
  /// where it was, and this is where that is put right.
  it("rewrites a registration that names somewhere else", () => {
    const starts = startup(packed(dir), noLogin);
    starts.set(true);
    writeFileSync(
      entryIn(dir),
      "[Desktop Entry]\nType=Application\nName=Verkstead\n" +
        'Exec="/where/it/used/to/be/verkstead" --hidden\n',
      "utf8",
    );

    starts.refresh();

    const registered = readFileSync(entryIn(dir), "utf8");
    expect(registered).toContain('Exec="/opt/verkstead/verkstead"');
    expect(registered).not.toContain("/where/it/used/to/be/");
  });

  /// And one of a machine nobody asked to be started on registers nothing: what
  /// a launch rewrites is what somebody asked for.
  it("registers nothing that was not registered", () => {
    const starts = startup(packed(dir), noLogin);

    starts.refresh();

    expect(starts.standing().on).toBe(false);
    expect(existsSync(entryIn(dir))).toBe(false);
  });

  /// Including the one whose entry a desktop turned off, which is an unchecked
  /// box: rewriting it would be Verkstead putting back what somebody took away.
  it("leaves an entry a desktop turned off exactly as it is", () => {
    const off = "[Desktop Entry]\nType=Application\nX-GNOME-Autostart-enabled=false\n";
    const starts = startup(packed(dir), noLogin);
    starts.set(true);
    writeFileSync(entryIn(dir), off, "utf8");

    starts.refresh();

    expect(readFileSync(entryIn(dir), "utf8")).toBe(off);
  });
});

describe("the login-item arm", () => {
  /// Read from the platform through the same call it is written with, which is
  /// what makes the registration the state on these two as well.
  it("reads the box off the API", () => {
    expect(startup(packed(dir, { platform: "darwin" }), loginItem(true)).standing()).toEqual({
      possible: true,
      on: true,
    });
    expect(startup(packed(dir, { platform: "win32" }), loginItem()).standing()).toEqual({
      possible: true,
      on: false,
    });
  });

  /// And the hidden start is that API's own: a Mac has a word for it, and
  /// Windows takes the flag on the command line the way the entry on Linux does.
  it("registers a login start that comes up hidden", () => {
    const login = loginItem();
    const starts = startup(packed(dir, { platform: "darwin" }), login);

    expect(starts.set(true)).toEqual({ possible: true, on: true });
    expect(login.asked).toEqual([{ openAtLogin: true, openAsHidden: true, args: [HIDDEN] }]);

    expect(starts.set(false).on).toBe(false);
    expect(login.asked[1]?.openAtLogin).toBe(false);
  });

  /// The rewrite-while-present, which on these two is the same call again: a
  /// bundle that was moved is re-registered where it is now.
  it("registers again at a launch while it is on, and not while it is off", () => {
    const on = loginItem(true);
    startup(packed(dir, { platform: "win32" }), on).refresh();
    expect(on.asked).toHaveLength(1);

    const off = loginItem();
    startup(packed(dir, { platform: "win32" }), off).refresh();
    expect(off.asked).toHaveLength(0);
  });

  /// And what the API refused is carried up to the page rather than thrown at
  /// it, exactly as a file that could not be written is.
  it("says what the API refused", () => {
    const login: LoginItem = {
      registered: () => false,
      register: () => {
        throw new Error("the login item could not be written");
      },
    };

    const now = startup(packed(dir, { platform: "darwin" }), login).set(true);

    expect(now).toEqual({
      possible: true,
      on: false,
      refused: expect.stringContaining("could not"),
    });
  });
});

describe("a login start", () => {
  const tray = (trayIcon: boolean): Settings => ({ whenClosed: "tray", trayIcon });

  /// The reading `--no-open` made of a login for the tray app: there is an icon
  /// to reach Verkstead by, so no window arrives over whatever the human is
  /// doing.
  it("comes up hidden while the tray is shown", () => {
    expect(hidden(["/opt/verkstead/verkstead", HIDDEN], tray(true))).toBe(true);
  });

  /// And comes up with a window where there is not, which is what keeps this a
  /// **Launch on Startup** rather than one that needs the tray: an app with no
  /// icon and no window is a Verkstead nobody can reach.
  it("comes up shown where there is no icon to reach it by", () => {
    expect(hidden(["/opt/verkstead/verkstead", HIDDEN], tray(false))).toBe(false);
  });

  /// And a launch by hand is a window whatever the tray says: somebody who
  /// started Verkstead is asking for it.
  it("is nothing a launch without the flag does", () => {
    expect(hidden(["/opt/verkstead/verkstead"], tray(true))).toBe(false);
    expect(hidden(["/usr/bin/electron", "."], tray(false))).toBe(false);
  });
});
