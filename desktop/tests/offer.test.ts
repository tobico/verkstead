//! The **Install Offer**: when a launch makes it, what it says, and what
//! **Install** writes where.
//!
//! **The failure this is here to catch is an offer at the wrong moment** — over
//! a login's start nobody is at, on a checkout run where there is no file of the
//! human's to move, or to somebody who already installed the app or ticked the
//! box — and an install that lands something a desktop will not draw: an entry
//! naming the download rather than the copy, or an `Icon` naming nothing that
//! was copied. So each no is asked for its own reason, and the entry and the
//! icon plan are pinned rather than described.

import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { beforeEach, describe, expect, it } from "vitest";

import {
  APPLICATIONS,
  asking,
  bundle,
  CATEGORIES,
  entry,
  failure,
  FILENAME,
  HICOLOR,
  icons,
  install,
  INSTALLING,
  KEYWORDS,
  moveInto,
  NotInstalled,
  offered,
  type Placing,
  placed,
  plan,
  relaunched,
  shown,
} from "../src/offer.js";
import { DEFAULTS } from "../src/settings.js";
import { APP_ID } from "../src/startup.js";

const HOME = "/home/ada";

/// A packed AppImage run from `appimage`, on a Linux box whose home is
/// [`HOME`].
const packed = (appimage: string | undefined, over: Partial<Placing> = {}): Placing => ({
  packaged: true,
  platform: "linux",
  exe: "/tmp/.mount_Verkst/Verkstead",
  applied: false,
  env: { HOME, ...(appimage === undefined ? {} : { APPIMAGE: appimage }) },
  ...over,
});

describe("where a run is", () => {
  it("is outside Applications run from Downloads", () => {
    expect(placed(packed("/home/ada/Downloads/Verkstead-x86_64.AppImage"))).toEqual({
      outside: "/home/ada/Downloads/Verkstead-x86_64.AppImage",
    });
  });

  /// One rule rather than Downloads by path: anywhere outside is outside.
  it("is outside Applications run from anywhere else", () => {
    for (const appimage of [
      "/home/ada/bin/Verkstead.AppImage",
      "/home/ada/ApplicationsOld/Verkstead-x86_64.AppImage",
      "/home/ada/Applications",
      "/opt/Verkstead-x86_64.AppImage",
    ]) {
      expect(placed(packed(appimage)), appimage).toEqual({ outside: appimage });
    }
  });

  /// Where this app installs to, and where AppImageLauncher and Gear Lever put
  /// theirs: a human who already integrated it is left alone.
  it("is inside Applications under ~/Applications, however deep", () => {
    for (const appimage of [
      "/home/ada/Applications/Verkstead-x86_64.AppImage",
      "/home/ada/Applications/verkstead_abc123.appimage",
      "/home/ada/Applications/gear-lever/Verkstead.AppImage",
    ]) {
      expect(placed(packed(appimage)), appimage).toEqual({ inside: appimage });
    }
  });

  it("is nowhere on a checkout run", () => {
    const where = placed(packed("/home/ada/Downloads/Verkstead-x86_64.AppImage", { packaged: false }));

    expect(where).toEqual({ nowhere: expect.stringContaining("checkout") });
  });

  it("is nowhere on Windows, whose download is the installer", () => {
    const where = placed(packed(undefined, { platform: "win32" }));

    expect(where).toEqual({ nowhere: expect.stringContaining("installer") });
  });

  /// A packed Linux run that is not an AppImage, or one whose variable is not
  /// a path, has no file of the human's to move.
  it("is nowhere without an absolute $APPIMAGE", () => {
    for (const appimage of [undefined, "", "Verkstead-x86_64.AppImage"]) {
      expect(placed(packed(appimage)), String(appimage)).toEqual({
        nowhere: expect.stringContaining("not an AppImage"),
      });
    }
  });

  /// The home read by the autostart directory's rule: absolute, or nothing.
  it("is nowhere without a home to install into", () => {
    for (const home of [undefined, "", "ada"]) {
      const where = placed({
        packaged: true,
        platform: "linux",
        exe: "/tmp/.mount_Verkst/Verkstead",
        applied: false,
        env: { APPIMAGE: "/tmp/Verkstead-x86_64.AppImage", ...(home === undefined ? {} : { HOME: home }) },
      });

      expect(where, String(home)).toEqual({ nowhere: expect.stringContaining("home") });
    }
  });
});

describe("where a Mac run is", () => {
  /// A packed bundle run from `exe`, with Electron's own answer about it.
  const mac = (exe: string, applied: boolean, over: Partial<Placing> = {}): Placing => ({
    packaged: true,
    platform: "darwin",
    env: { HOME: "/Users/ada" },
    exe,
    applied,
    ...over,
  });

  /// Electron's `isInApplicationsFolder` is the whole reading: the dmg, the
  /// Downloads folder and Gatekeeper's translocated path are all a no.
  it("is outside Applications wherever Electron says it is not in one", () => {
    for (const app of [
      "/Volumes/Verkstead/Verkstead.app",
      "/Users/ada/Downloads/Verkstead.app",
      "/private/var/folders/xy/abc/T/AppTranslocation/1F2E/d/Verkstead.app",
    ]) {
      expect(placed(mac(`${app}/Contents/MacOS/Verkstead`, false)), app).toEqual({ outside: app });
    }
  });

  it("is inside Applications where Electron says it is in one", () => {
    expect(placed(mac("/Applications/Verkstead.app/Contents/MacOS/Verkstead", true))).toEqual({
      inside: "/Applications/Verkstead.app",
    });
  });

  /// `$APPIMAGE` is Linux's reading and means nothing here.
  it("reads no $APPIMAGE", () => {
    const where = placed(
      mac("/Volumes/Verkstead/Verkstead.app/Contents/MacOS/Verkstead", false, {
        env: { HOME: "/Users/ada", APPIMAGE: "/Users/ada/Applications/Verkstead-x86_64.AppImage" },
      }),
    );

    expect(where).toEqual({ outside: "/Volumes/Verkstead/Verkstead.app" });
  });

  it("is nowhere on a checkout run, whatever Electron says", () => {
    const where = placed(mac("/nix/store/x-electron/Electron.app/Contents/MacOS/Electron", false, { packaged: false }));

    expect(where).toEqual({ nowhere: expect.stringContaining("checkout") });
  });

  it("is offered to a download run by hand, and waits out a login's start", () => {
    const where = placed(mac("/Volumes/Verkstead/Verkstead.app/Contents/MacOS/Verkstead", false));

    expect(offered(where, () => false, DEFAULTS)).toEqual({ offer: "/Volumes/Verkstead/Verkstead.app" });
    expect(offered(where, () => true, DEFAULTS)).toEqual({ why: expect.stringContaining("login") });
    expect(offered(where, () => false, { ...DEFAULTS, installOffer: false })).toEqual({
      why: "the desktop settings say don't ask again",
    });
  });

  it("names the bundle an executable is inside", () => {
    expect(bundle("/Applications/Verkstead.app/Contents/MacOS/Verkstead")).toBe("/Applications/Verkstead.app");
    expect(bundle("/usr/local/bin/Verkstead")).toBe("/usr/local/bin/Verkstead");
  });
});

describe("whether the offer is made", () => {
  const download = placed(packed("/home/ada/Downloads/Verkstead-x86_64.AppImage"));

  it("is made to a download run by hand", () => {
    expect(offered(download, () => false, DEFAULTS)).toEqual({
      offer: "/home/ada/Downloads/Verkstead-x86_64.AppImage",
    });
  });

  /// Each for its own reason, which the log says.
  it("is not made on a checkout run", () => {
    const where = placed(packed("/home/ada/Downloads/Verkstead-x86_64.AppImage", { packaged: false }));

    expect(offered(where, () => false, DEFAULTS)).toEqual({ why: expect.stringContaining("checkout") });
  });

  it("is not made to an app already in Applications", () => {
    const where = placed(packed("/home/ada/Applications/Verkstead-x86_64.AppImage"));

    expect(offered(where, () => false, DEFAULTS)).toEqual({
      why: "it already runs from /home/ada/Applications/Verkstead-x86_64.AppImage",
    });
  });

  it("waits out a hidden start", () => {
    expect(offered(download, () => true, DEFAULTS)).toEqual({ why: expect.stringContaining("login") });
  });

  it("is not made once the box is ticked", () => {
    expect(offered(download, () => false, { ...DEFAULTS, installOffer: false })).toEqual({
      why: "the desktop settings say don't ask again",
    });
  });

  /// Asked only of a run outside Applications: a Mac's reading of whether its
  /// login item started this run is not one to make for nothing.
  it("asks whether the start is hidden only of a download", () => {
    const asked = { count: 0 };
    const unseen = () => {
      asked.count += 1;
      return false;
    };

    offered(placed(packed(undefined, { packaged: false })), unseen, DEFAULTS);
    offered(placed(packed("/home/ada/Applications/Verkstead-x86_64.AppImage")), unseen, DEFAULTS);
    expect(asked.count).toBe(0);

    offered(download, unseen, DEFAULTS);
    expect(asked.count).toBe(1);
  });

  /// A checkout is a checkout whatever else is true: the reason said is the
  /// one that matters most.
  it("says the checkout before the login or the box", () => {
    const where = placed(packed(undefined, { packaged: false }));

    expect(offered(where, () => true, { ...DEFAULTS, installOffer: false })).toEqual({
      why: expect.stringContaining("checkout"),
    });
  });
});

describe("the relaunch", () => {
  it("carries this launch's arguments without the executable or --install", () => {
    expect(relaunched(["/tmp/.mount_x/Verkstead", "--install", "--enable-logging"])).toEqual([
      "--enable-logging",
    ]);
    expect(relaunched(["/tmp/.mount_x/Verkstead"])).toEqual([]);
  });
});

describe("the dialog", () => {
  const machine = { platform: "linux" as const, env: { HOME } };

  /// ADR-0020's words, pinned.
  it("asks, says where it runs from and what installing does, and offers the box", () => {
    const asked = asking("/home/ada/Downloads/Verkstead-x86_64.AppImage", machine);

    expect(asked).toEqual({
      message: "Install Verkstead?",
      detail:
        "Verkstead is running from ~/Downloads. Installing moves it to ~/Applications, " +
        "adds it to your applications menu, and restarts it from there.",
      buttons: ["Install", "Skip"],
      defaultId: 0,
      cancelId: 1,
      checkboxLabel: "Don't ask again",
    });
    expect(asked.buttons[INSTALLING]).toBe("Install");
  });

  it("names a folder outside the home as it is", () => {
    expect(asking("/media/usb/Verkstead.AppImage", machine).detail).toContain(
      "running from /media/usb.",
    );
  });

  it("says why an install failed, and that the app carries on", () => {
    expect(
      failure(
        "It couldn't be moved to ~/Applications (ENOSPC: no space left on device).",
        "/home/ada/Downloads/Verkstead-x86_64.AppImage",
        machine,
      ),
    ).toEqual({
      title: "Verkstead couldn't be installed.",
      content:
        "It couldn't be moved to ~/Applications (ENOSPC: no space left on device).\n\n" +
        "It carries on running from ~/Downloads.",
    });
  });

  /// The Mac's words are ADR-0020's too, and name no path: a translocated app
  /// runs from a folder nobody chose.
  it("says a Mac's in the Mac's words", () => {
    const mac = { platform: "darwin" as const, env: { HOME: "/Users/ada" } };

    expect(asking("/Volumes/Verkstead/Verkstead.app", mac)).toEqual({
      message: "Install Verkstead?",
      detail:
        "Verkstead is running from outside Applications. " +
        "Installing moves it into Applications and opens it from there.",
      buttons: ["Install", "Skip"],
      defaultId: 0,
      cancelId: 1,
      checkboxLabel: "Don't ask again",
    });
    expect(failure("Moving it into Applications was cancelled.", "/Volumes/Verkstead/Verkstead.app", mac)).toEqual({
      title: "Verkstead couldn't be installed.",
      content: "Moving it into Applications was cancelled.\n\nIt carries on running from outside Applications.",
    });
  });

  it("writes a path under the home from ~", () => {
    expect(shown("/home/ada", machine)).toBe("~");
    expect(shown("/home/ada/Downloads", machine)).toBe("~/Downloads");
    expect(shown("/home/adam/Downloads", machine)).toBe("/home/adam/Downloads");
  });
});

describe("the installed entry", () => {
  /// Pinned: an entry naming the download, or an `Icon` naming nothing that was
  /// copied, is an applications menu with a broken or blank Verkstead in it.
  it("names the copy, quoted, with the packed entry's fields", () => {
    expect(entry("/home/ada/Applications/Verkstead-x86_64.AppImage")).toBe(
      "[Desktop Entry]\n" +
        "Type=Application\n" +
        "Name=Verkstead\n" +
        "Comment=The workbench, in the system tray\n" +
        'Exec="/home/ada/Applications/Verkstead-x86_64.AppImage" %U\n' +
        "Icon=net.tobico.Verkstead\n" +
        "Terminal=false\n" +
        "StartupWMClass=net.tobico.Verkstead\n" +
        "Categories=Development;\n" +
        "Keywords=agent;coding;sessions;workbench;\n",
    );
  });

  it("quotes a path the way the autostart entry does", () => {
    expect(entry("/home/a b/Applications/Verkstead-x86_64.AppImage")).toContain(
      'Exec="/home/a b/Applications/Verkstead-x86_64.AppImage" %U\n',
    );
    expect(entry('/home/$x/Applications/Verkstead-x86_64.AppImage')).toContain(
      'Exec="/home/\\\\$x/Applications/Verkstead-x86_64.AppImage" %U\n',
    );
  });

  /// Said again here because this app writes the entry rather than the pack —
  /// so the yml is read for what it says, and the two cannot drift apart.
  it("carries what electron-builder.yml gives the packed entry", () => {
    const yml = readFileSync(join(import.meta.dirname, "..", "electron-builder.yml"), "utf8");

    expect(yml).toMatch(new RegExp(`^  category: ${CATEGORIES.replace(/;$/, "")}$`, "m"));
    expect(yml).toMatch(new RegExp(`^      Keywords: ${KEYWORDS}$`, "m"));
    expect(yml).toMatch(/^  description: The workbench, in the system tray$/m);
    expect(yml).toMatch(/^appId: net\.tobico\.Verkstead$/m);
    expect(yml).toMatch(new RegExp(`^  artifactName: ${FILENAME.replace(/\./g, "\\.")}$`, "m"));
  });
});

describe("the icon plan", () => {
  /// electron-builder's own layout inside the image, renamed to what the entry
  /// names.
  it("copies every size of the app's icon, named for the app id", () => {
    expect(
      icons(
        "/tmp/.mount_x/usr/share/icons/hicolor",
        ["16x16/apps/Verkstead.png", "256x256/apps/Verkstead.png", "scalable/apps/Verkstead.svg"],
        "/home/ada/.local/share/icons/hicolor",
      ),
    ).toEqual([
      {
        from: "/tmp/.mount_x/usr/share/icons/hicolor/16x16/apps/Verkstead.png",
        to: `/home/ada/.local/share/icons/hicolor/16x16/apps/${APP_ID}.png`,
      },
      {
        from: "/tmp/.mount_x/usr/share/icons/hicolor/256x256/apps/Verkstead.png",
        to: `/home/ada/.local/share/icons/hicolor/256x256/apps/${APP_ID}.png`,
      },
      {
        from: "/tmp/.mount_x/usr/share/icons/hicolor/scalable/apps/Verkstead.svg",
        to: `/home/ada/.local/share/icons/hicolor/scalable/apps/${APP_ID}.svg`,
      },
    ]);
  });

  it("leaves what is not an application icon", () => {
    expect(
      icons("/t", ["16x16", "16x16/apps", "16x16/mimetypes/x.png", "16x16/apps/notes.txt", "index.theme"], "/h"),
    ).toEqual([]);
  });
});

describe("the plan", () => {
  it("moves the file into ~/Applications under the release's name, with the entry and icons", () => {
    const planned = plan(
      "/home/ada/Downloads/Verkstead-x86_64 (1).AppImage",
      { platform: "linux", env: { HOME } },
      "/tmp/.mount_x",
      ["64x64/apps/Verkstead.png"],
    );

    expect(planned).toEqual({
      from: "/home/ada/Downloads/Verkstead-x86_64 (1).AppImage",
      dir: `/home/ada/${APPLICATIONS}`,
      file: "/home/ada/Applications/Verkstead-x86_64.AppImage",
      entry: "/home/ada/.local/share/applications/net.tobico.Verkstead.desktop",
      text: entry("/home/ada/Applications/Verkstead-x86_64.AppImage"),
      icons: [
        {
          from: `/tmp/.mount_x/${HICOLOR}/64x64/apps/Verkstead.png`,
          to: "/home/ada/.local/share/icons/hicolor/64x64/apps/net.tobico.Verkstead.png",
        },
      ],
    });
  });

  /// The desktop's own data home, read the specification's way.
  it("writes the entry and icons under $XDG_DATA_HOME where it is absolute", () => {
    const planned = plan("/d/V.AppImage", { platform: "linux", env: { HOME, XDG_DATA_HOME: "/xdg" } }, "/m", [
      "32x32/apps/Verkstead.png",
    ]);

    expect(planned?.entry).toBe("/xdg/applications/net.tobico.Verkstead.desktop");
    expect(planned?.icons[0]?.to).toBe("/xdg/icons/hicolor/32x32/apps/net.tobico.Verkstead.png");
  });

  it("copies no icons with no mount to copy them from", () => {
    expect(plan("/d/V.AppImage", { platform: "linux", env: { HOME } }, undefined, ["x"])?.icons).toEqual([]);
  });
});

describe("installing", () => {
  let root: string;
  let machine: { platform: NodeJS.Platform; env: Record<string, string> };

  beforeEach(() => {
    root = mkdtempSync(join(tmpdir(), "verkstead-offer-"));
    machine = { platform: "linux", env: { HOME: join(root, "home") } };
    mkdirSync(join(root, "home", "Downloads"), { recursive: true });
    mkdirSync(join(root, "mount", HICOLOR, "128x128", "apps"), { recursive: true });
    writeFileSync(join(root, "mount", HICOLOR, "128x128", "apps", "Verkstead.png"), "png");
    writeFileSync(join(root, "home", "Downloads", "Verkstead.AppImage"), "image", { mode: 0o644 });
  });

  it("lands the file, executable, the entry and the icons, and leaves no download", () => {
    const from = join(root, "home", "Downloads", "Verkstead.AppImage");
    const planned = plan(from, machine, join(root, "mount"), ["128x128/apps/Verkstead.png"]);
    if (planned === undefined) {
      throw new Error("no plan");
    }

    install(planned, machine);

    const file = join(root, "home", "Applications", FILENAME);
    expect(readFileSync(file, "utf8")).toBe("image");
    expect(statSync(file).mode & 0o111).toBe(0o111);
    expect(existsSync(from)).toBe(false);
    expect(
      readFileSync(join(root, "home", ".local", "share", "applications", `${APP_ID}.desktop`), "utf8"),
    ).toBe(entry(file));
    expect(
      readFileSync(join(root, "home", ".local", "share", "icons", "hicolor", "128x128", "apps", `${APP_ID}.png`), "utf8"),
    ).toBe("png");
  });

  /// A move that could not be made is a dialog saying why, and nothing is left
  /// naming a file that is not there.
  it("says why, worded for the dialog, and writes no entry when the move fails", () => {
    const planned = plan(join(root, "home", "Downloads", "gone.AppImage"), machine, undefined, []);
    if (planned === undefined) {
      throw new Error("no plan");
    }

    expect(() => install(planned, machine)).toThrow(NotInstalled);
    expect(() => install(planned, machine)).toThrow(/^It couldn't be moved to ~\/Applications \(ENOENT/);
    expect(existsSync(planned.entry)).toBe(false);
  });
});

describe("a Mac's move", () => {
  it("is done when Electron moved the bundle", () => {
    expect(() => moveInto(() => true)).not.toThrow();
  });

  /// The password dialog cancelled: the human pressed Install, so it is said.
  it("says a cancelled move was cancelled", () => {
    expect(() => moveInto(() => false)).toThrow(new NotInstalled("Moving it into Applications was cancelled."));
  });

  it("says why a move that threw could not be made", () => {
    const move = () => {
      throw new Error("Failed to copy app");
    };

    expect(() => moveInto(move)).toThrow(NotInstalled);
    expect(() => moveInto(move)).toThrow("It couldn't be moved into Applications (Failed to copy app).");
  });
});
