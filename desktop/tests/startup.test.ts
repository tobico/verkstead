//! **Launch on Startup**: where the registration goes, what it says, and what
//! reading it back comes to.
//!
//! The Rust tray app's own suite, arm for arm, because the
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
//! rather than a file — so they are exercised through a stub of the calls this
//! needs of it, which is what makes the arm this Linux runner will never run an
//! ordinary test all the same. The plist the tray app left on a Mac is the one
//! thing there that *is* a file, and it is `launchd.test.ts`'s.

import { existsSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { type Registry, RUN } from "../src/registry.js";
import type { Settings } from "../src/settings.js";
import {
  APP_ID,
  ARGS,
  autostartDir,
  HIDDEN,
  hidden,
  named,
  quoted,
  type LoginAsked,
  type LoginItem,
  type LoginStatus,
  type Registered,
  type Registering,
  saysOn,
  startup,
  type Startup,
  switchedOff,
  VALUE,
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

/// Electron's login-item API, stood in for: what it was asked, and what it
/// answers next.
///
/// **It answers the way both platforms do, which is not the same way** — that
/// being the whole of what it is for beyond recording. A Mac's `openAtLogin` is
/// the registration as a yes or a no, and `registered` answers that. Windows'
/// half of it is a list of the values under the Run key that name this
/// executable, and `values` answers that: under whatever name the write carried,
/// so that a reading which looked for the wrong one finds nothing here either.
/// The other half is [`registryOf`], which is where a value that is there at all
/// comes from.
function loginItem(
  on = false,
  args: string[] = [...ARGS],
  opened = false,
  status: LoginStatus | undefined = undefined,
  named = VALUE,
  enabled = true,
): LoginItem & { asked: LoginAsked[]; written: () => LoginAsked | undefined } {
  const asked: LoginAsked[] = [];
  let registered: LoginAsked | undefined = on
    ? { openAtLogin: true, args, name: named }
    : undefined;

  return {
    asked,
    written: () => registered,
    registered: () => registered !== undefined,
    values: () =>
      registered === undefined ? [] : [{ name: registered.name, enabled }],
    openedAtLogin: () => opened,
    status: () => status,
    register: (wanted) => {
      asked.push(wanted);
      registered = wanted.openAtLogin ? wanted : undefined;
    },
  };
}

/// The registry Windows' half of the reading is asked of, stood in for: whether
/// Verkstead's own value is under the Run key, which is a question about the value
/// rather than about what it names.
///
/// **Read off the login item's own record rather than kept beside it**, because a
/// machine has one Run key: what the API wrote is what `reg.exe` would find, and a
/// stub with two records of that could agree when a machine would not. `has` is
/// the only call the reading makes; a `remove` here would be the take-over's, and
/// that is `runkey.test.ts`'s.
const registryOf = (login: { written: () => LoginAsked | undefined }): Registry => ({
  has: (key, value) => key === RUN && login.written()?.name === value,
  remove: () => {
    throw new Error("the reading does not delete anything");
  },
});

/// Verkstead's startup on a Windows machine whose Run key holds what `login` was
/// told to write — the one arm whose reading is two calls rather than one, so the
/// one arm with a third argument.
const windows = (login: ReturnType<typeof loginItem>): Startup =>
  startup(packed(dir, { platform: "win32" }), login, registryOf(login));

/// One that is never called, for the arms that are a file rather than a call.
const noLogin: LoginItem = {
  registered: (): boolean => {
    throw new Error("the Linux arm is a file, and asked the login-item API");
  },
  values: (): Registered[] => {
    throw new Error("the Linux arm is a file, and asked the login-item API");
  },
  openedAtLogin: (): boolean => {
    throw new Error("the Linux arm is a file, and asked the login-item API");
  },
  status: (): LoginStatus => {
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
    expect(windows(loginItem()).standing()).toEqual({
      possible: true,
      on: false,
    });
  });

  /// And the hidden start is not asked of the API at all: `openAsHidden` was a
  /// Mac's word for it and has been deprecated and inert since macOS 13, so what
  /// is sent is the flag Windows reads on the command line — and a Mac is asked
  /// afterwards whether its login item started the run, which is the describe
  /// below.
  it("registers a login start that comes up hidden", () => {
    const login = loginItem();
    const starts = startup(packed(dir, { platform: "darwin" }), login);

    expect(starts.set(true)).toEqual({ possible: true, on: true });
    expect(login.asked).toEqual([{ openAtLogin: true, args: [HIDDEN], name: VALUE }]);

    expect(starts.set(false).on).toBe(false);
    expect(login.asked[1]?.openAtLogin).toBe(false);
  });

  /// The rewrite-while-present, which on these two is the same call again: a
  /// bundle that was moved is re-registered where it is now.
  it("registers again at a launch while it is on, and not while it is off", () => {
    const on = loginItem(true);
    windows(on).refresh();
    expect(on.asked).toHaveLength(1);

    const off = loginItem();
    windows(off).refresh();
    expect(off.asked).toHaveLength(0);
  });

  /// **Windows writes a value of its own name and is read back by it**, because
  /// the reading that platform has for `openAtLogin` looks at one name and it is
  /// not this one — the AppUserModelId's, which is the tray app's value and
  /// `runkey.ts`'s to take over. A box read through that call springs back the
  /// moment it is ticked, on a machine that really will start Verkstead at the
  /// next sign-in, and never rewrites the registration at a launch.
  it("reads the box back off the value it wrote", () => {
    const login = loginItem();
    const starts = windows(login);

    expect(starts.set(true)).toEqual({ possible: true, on: true });
    expect(login.asked).toEqual([{ openAtLogin: true, args: ARGS, name: VALUE }]);
    expect(login.values()).toEqual([{ name: VALUE, enabled: true }]);
  });

  /// And a value naming this executable that this app did not write is not this
  /// app's registration: the tray app's own is exactly that, and reading it as a
  /// tick is a box that says on about a binary the upgrade deleted.
  it("reads no box off a value written under another name", () => {
    const login = loginItem(true, [...ARGS], false, undefined, APP_ID);

    expect(windows(login).standing()).toEqual({ possible: true, on: false });
  });

  /// **And a value naming somewhere else is still the registration**, which is
  /// the arm the whole two-part reading exists for: Electron lists only the
  /// values naming the executable that is running, so a machine whose app has
  /// moved would read as unregistered and the launch that could heal it would
  /// leave it alone — see `LoginItem.values`. The registry sees the value
  /// whatever it names, and the rewrite is what puts it right.
  it("reads the box on off a value the API cannot see, and rewrites it", () => {
    const login = loginItem(true);
    const moved: Registry = { has: () => true, remove: () => undefined };
    const starts = startup(
      packed(dir, { platform: "win32" }),
      { ...login, values: () => [] },
      moved,
    );

    expect(starts.standing()).toEqual({ possible: true, on: true });

    starts.refresh();
    expect(login.asked).toEqual([{ openAtLogin: true, args: ARGS, name: VALUE }]);
  });

  /// **And a value Task Manager switched off is an unchecked box**, which is the
  /// same reading the Linux entry's two off-keys get: Explorer keeps that answer
  /// in a key of its own, Electron reads it back beside the value, and turning
  /// Verkstead off there *is* unticking this. Which is also what keeps the
  /// rewrite-at-a-launch from putting back what somebody took away — a
  /// registration that reads off is one `refresh` leaves alone, and every write
  /// through this API clears that record.
  it("reads no box off a value switched off in Task Manager", () => {
    const login = loginItem(true, [...ARGS], false, undefined, VALUE, false);
    const starts = windows(login);

    expect(starts.standing()).toEqual({ possible: true, on: false });

    starts.refresh();
    expect(login.asked).toEqual([]);
  });

  /// And what the API refused is carried up to the page rather than thrown at
  /// it, exactly as a file that could not be written is.
  it("says what the API refused", () => {
    const login: LoginItem = {
      registered: () => false,
      values: () => [],
      openedAtLogin: () => false,
      status: () => undefined,
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

describe("Verkstead's own value among the Run values Windows reported", () => {
  /// **The one thing nothing else protects.** Electron names the value it writes
  /// after the AppUserModelId where it is told nothing, and `main.ts` sets that
  /// to the app id so the window groups with the Start-menu entry — which is the
  /// name `runkey.ts` reads as the tray app's and deletes. One name for both and
  /// the launch after a take-over reads its own registration as that app's,
  /// registers again and removes it: a box that unregisters itself every other
  /// launch.
  it("is not the name the take-over reads", () => {
    expect(VALUE).not.toBe(APP_ID);
  });

  /// Windows' own way of turning a value off rather than deleting it — the twin
  /// of the two keys `saysOn` reads, and Verkstead's own value alone.
  it("is switched off where Explorer says the value is not approved", () => {
    expect(switchedOff([{ name: VALUE, enabled: false }])).toBe(true);
    expect(switchedOff([{ name: VALUE, enabled: true }])).toBe(false);
    expect(switchedOff([])).toBe(false);
    expect(switchedOff([{ name: APP_ID, enabled: false }])).toBe(false);
    expect(
      switchedOff([
        { name: "Something Else", enabled: true },
        { name: VALUE, enabled: false },
      ]),
    ).toBe(true);
  });
});

describe("a registration macOS is holding", () => {
  /// The state `SMAppService` reports beside `openAtLogin`: the registration is
  /// still there, and `requires-approval` is the platform saying it will not
  /// start Verkstead until somebody approves it. So the box is greyed with what
  /// the human can actually do named under it — the row under *Open at Login*
  /// and its minus button, which is what a real Mac had on it — rather than a
  /// tick that writes a registration the system is already ignoring.
  it("is a box that cannot be ticked, and says what to do about it", () => {
    const standing = startup(
      packed(dir, { platform: "darwin" }),
      loginItem(false, [...ARGS], false, "requires-approval"),
    ).standing();

    expect(standing.possible).toBe(false);
    expect(standing.on).toBe(false);
    // Both halves of the note, because the pane on its own is where the first
    // wording sent a human to look for a switch that is not on that row.
    expect(standing.why).toMatch(/System Settings/);
    expect(standing.why).toMatch(/minus button/);
  });

  /// And every other thing that status says is the ordinary reading: a
  /// registration that is there and enabled is a ticked box, and the two that
  /// say there is none are what `openAtLogin` already says.
  it("is nothing the other three states do", () => {
    for (const status of ["enabled", "not-registered", "not-found"] as const) {
      expect(
        startup(
          packed(dir, { platform: "darwin" }),
          loginItem(true, [...ARGS], false, status),
        ).standing(),
        status,
      ).toEqual({ possible: true, on: true });
    }
  });

  /// Nor anything a Mac before 13 says, which is nothing at all: that platform
  /// has no `SMAppService` to be held by.
  it("is nothing where the platform says nothing", () => {
    expect(startup(packed(dir, { platform: "darwin" }), loginItem(true)).standing()).toEqual({
      possible: true,
      on: true,
    });
  });

  /// And it is not asked of Windows at all — a status is a Mac's word, and the
  /// stub that throws when it is asked is what says so.
  it("is not asked of Windows", () => {
    const login = {
      ...loginItem(true),
      status: (): LoginStatus => {
        throw new Error("Windows has no SMAppService, and was asked for a status");
      },
    };

    expect(windows(login).standing()).toEqual({ possible: true, on: true });
  });
});

describe("a Mac's own account of a login start", () => {
  /// What that platform has instead of the flag: its login item carries no
  /// arguments — `args` is Windows' alone — so `--hidden` never reaches
  /// `process.argv` there and the platform is asked directly.
  it("is the login item saying it started this run", () => {
    const starts = startup(packed(dir, { platform: "darwin" }), loginItem(true, [...ARGS], true));

    expect(starts.atLogin()).toBe(true);
  });

  it("is nothing where the login item did not", () => {
    expect(startup(packed(dir, { platform: "darwin" }), loginItem(true)).atLogin()).toBe(false);
  });

  /// And nothing the other two are asked, their registrations being command
  /// lines: a stub that throws when it is asked is what says so.
  it("is not asked of Linux, whose entry carries the flag", () => {
    expect(startup(packed(dir), noLogin).atLogin()).toBe(false);
  });

  it("is not asked of Windows, whose Run key carries it", () => {
    expect(windows(loginItem(true, [...ARGS], true)).atLogin()).toBe(false);
  });

  /// Nor of a machine with nowhere to keep a registration: one that was never
  /// made started nothing.
  it("is nothing on an unpackaged run", () => {
    expect(
      startup(packed(dir, { platform: "darwin", packaged: false }), noLogin).atLogin(),
    ).toBe(false);
  });
});

describe("a login start", () => {
  const tray = (trayIcon: boolean): Settings => ({ whenClosed: "tray", trayIcon });

  /// The reading `--no-open` made of a login for the tray app: there is an icon
  /// to reach Verkstead by, so no window arrives over whatever the human is
  /// doing.
  it("comes up hidden while the tray is shown", () => {
    expect(hidden(["/opt/verkstead/verkstead", HIDDEN], tray(true), false)).toBe(true);
  });

  /// And on a Mac, where the same thing is said by the platform rather than by
  /// the command line.
  it("comes up hidden where the platform says the login item started it", () => {
    expect(hidden(["/Applications/Verkstead.app/…/Verkstead"], tray(true), true)).toBe(true);
  });

  /// And comes up with a window where there is not, which is what keeps this a
  /// **Launch on Startup** rather than one that needs the tray: an app with no
  /// icon and no window is a Verkstead nobody can reach.
  it("comes up shown where there is no icon to reach it by", () => {
    expect(hidden(["/opt/verkstead/verkstead", HIDDEN], tray(false), false)).toBe(false);
    expect(hidden(["/Applications/Verkstead.app/…/Verkstead"], tray(false), true)).toBe(false);
  });

  /// And a launch by hand is a window whatever the tray says: somebody who
  /// started Verkstead is asking for it.
  it("is nothing a launch without the flag does", () => {
    expect(hidden(["/opt/verkstead/verkstead"], tray(true), false)).toBe(false);
    expect(hidden(["/usr/bin/electron", "."], tray(false), false)).toBe(false);
  });
});
