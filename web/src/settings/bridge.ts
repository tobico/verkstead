//! The app's own window, as the page reaches it: `window.verkstead`, or nothing
//! at all.
//!
//! **This is what makes the Desktop section the app's rather than the server's**
//! (ADR-0020, Set 847 Q12). What closing the window means and whether there is
//! an icon in the tray are facts about the desk in front of the human, so
//! nothing about them is on the wire and nothing about them is in `config.yaml`:
//! the page asks the window it is drawn in, over a preload script the Electron
//! app puts [`NAME`] on, and the app answers out of a JSON file of its own.
//!
//! **And a page with no bridge is a page with no Desktop section.** The same
//! document served to a browser on this machine, or to a phone over the tailnet,
//! carries no preload — so [`bridge`] answers `null` there, and every one of
//! this section's two halves draws nothing at all. That is the whole mechanism
//! by which a phone never sees it, and it is why a missing bridge is read as
//! *this is not the app* rather than as something that failed.
//!
//! **The shape is written twice on purpose.** `desktop/src/bridge.ts` is the
//! authority — it is what the preload exposes — and the two packages are built
//! apart, so there is nothing for this one to import. What keeps them in step is
//! the pair of tests that pin the same literals: `desktop/tests/bridge.test.ts`
//! on that side and `tests/desktop.test.tsx` on this one. A shape that drifted
//! would be a section drawn out of `undefined`, which is why what is read off
//! the window is checked for the shape rather than cast into it.
//!
//! **And one of the things it reaches is not a setting at all.**
//! [`Registration`] is read from the platform rather than out of the app's JSON
//! file: **Launch on Startup** is the registration itself (Set 846 Q9a), so there
//! is nothing kept beside it to disagree with, and a box drawn from it says what
//! the machine is actually going to do at the next login.
//!
//! What this module holds is the shape and the reading of the window, and
//! nothing else: the words the section is drawn in are `Desktop.tsx`'s, and what
//! a set *means* is the app's — see `changed` in `desktop/src/settings.ts`.

/// What the app puts the bridge on the window as: `window.verkstead`.
///
/// The product's name, because its absence is read as *not the app* rather than
/// as a missing feature. The same literal as `NAME` in `desktop/src/bridge.ts`,
/// and the two suites pin it on both sides.
export const NAME = "verkstead";

/// The platform a Mac reads as, which is the one the close radio is not drawn
/// on: closing a window there leaves the application running in the Dock, and
/// that is the platform's own answer rather than a position on any radio.
export const MAC = "darwin";

/// The positions of **When the window is closed**, in the order the radio draws
/// them: keep running in the tray, ask before quitting, quit.
///
/// A list rather than a union written out, so the radio is drawn from the same
/// three words the app checks a set against.
export const POSITIONS = ["tray", "ask", "quit"] as const;

/// One of the three.
export type WhenClosed = (typeof POSITIONS)[number];

/// Everything the app keeps about this desk — `Settings` in
/// `desktop/src/settings.ts`, said again on this side of the bridge.
export interface DesktopSettings {
  /// What a press of the window's close button means.
  whenClosed: WhenClosed;

  /// Whether there is an icon in the tray — and so whether the app can be
  /// reached at all once its window is off the screen.
  trayIcon: boolean;
}

/// How **Launch on Startup** stands on this machine — `Registration` in
/// `desktop/src/startup.ts`, said again on this side of the bridge.
///
/// **There is no setting for this anywhere**, which is the whole shape of it: the
/// platform's own registration is the state (Set 846 Q9a), so the box is drawn
/// from reading that registration and a tick writes it. A human who turns it off
/// with their desktop's own settings has unchecked the box.
export interface Registration {
  /// Whether this machine can be registered with at all. A greyed box rather
  /// than one that ticks and does nothing — an unpackaged run is the case a
  /// developer meets, and a machine with nowhere to keep an entry is the other.
  readonly possible: boolean;

  /// Whether Verkstead comes up when the desktop session does.
  readonly on: boolean;

  /// Why it cannot be, where it cannot — the note under the greyed box.
  readonly why?: string;

  /// And what refused a registration that was asked for, where this answers one.
  /// Said rather than thrown, so the line under the box is the app's wording
  /// rather than a bridge's.
  readonly refused?: string;
}

/// What `window.verkstead` is, where there is one.
export interface Bridge {
  /// Which platform the app is running on — `process.platform`, read in the
  /// preload where there is a process to read it from.
  ///
  /// A value rather than a call, because the page draws different controls for a
  /// Mac and one that had to await it would draw the wrong ones first.
  readonly platform: string;

  /// The settings as they stand on this machine.
  settings(): Promise<DesktopSettings>;

  /// Change what is named and leave the rest, answering with the settings in
  /// force afterwards — so what moves a control is the answer coming back
  /// rather than the press, and a refused set leaves it where it was.
  set(changed: Partial<DesktopSettings>): Promise<DesktopSettings>;

  /// Open this run's log file, or say there is none: the tray item's own act,
  /// reached the other way (ADR-0020).
  logs(): Promise<void>;

  /// How Launch on Startup stands, read from the platform's own registration.
  startup(): Promise<Registration>;

  /// Register or unregister, answering with how it stands afterwards — so what
  /// moves the box is the answer rather than the press, here as everywhere else
  /// on this page.
  register(on: boolean): Promise<Registration>;
}

/// The bridge this page is drawn over, or `null` where there is none — which is
/// every browser and every phone.
///
/// Read at the moment a component is built rather than followed: a preload is
/// there before the document is, so nothing can arrive or leave while a page is
/// open.
export function bridge(): Bridge | null {
  const held = (globalThis as Record<string, unknown>)[NAME];

  return shaped(held) ? held : null;
}

/// Whether what is on the window is the bridge.
///
/// Checked rather than cast, because a shape that had drifted from the app's
/// would draw this section out of `undefined` — and because the name is a name
/// in a page's whole namespace of them, so *something is there* is not the same
/// as *the app is there*.
function shaped(held: unknown): held is Bridge {
  if (typeof held !== "object" || held === null) {
    return false;
  }

  const reached = held as Record<string, unknown>;

  return (
    typeof reached.platform === "string" &&
    typeof reached.settings === "function" &&
    typeof reached.set === "function" &&
    typeof reached.logs === "function" &&
    typeof reached.startup === "function" &&
    typeof reached.register === "function"
  );
}
