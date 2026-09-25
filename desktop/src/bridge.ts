//! What the page can reach the app through, as a shape both sides read.
//!
//! **The bridge is how the Desktop page exists at all** (ADR-0020, Set 847
//! Q12). The settings it draws are the app's rather than the server's, so
//! nothing new goes on the wire to carry them: the page asks the window it is
//! drawn in, over a preload script that puts [`NAME`] on it, and the main
//! process answers out of the JSON file beside the window's remembered bounds.
//!
//! **And it is the app's window's alone.** The same document served to a
//! browser on this machine, or to a phone over the tailnet, carries no preload
//! and so has no `window.verkstead` — which is what the page reads as *this is
//! not the app*, rather than as something that failed. A page that finds
//! nothing here is a page with no Desktop section, and that is the whole
//! mechanism.
//!
//! **What is in this file is what crosses**: the name, the five channels, and
//! the shape. It holds no behaviour and touches no disk on purpose — the
//! preload imports it, so everything it imports is loaded inside the window,
//! and what the app *does* about each of them is `main.ts`'s while what they
//! *mean* is elsewhere: a set is [`changed`](./settings.js)'s, and the startup
//! registration is [`startup.ts`](./startup.js)'s.

import type { Settings } from "./settings.js";
import type { Registration } from "./startup.js";

/// What the bridge is called on the window object: `window.verkstead`.
///
/// The product's name, because the page reads its absence as *not the app*
/// rather than as a missing feature — and a page inside the app is already
/// looking at Verkstead whatever else is on that object.
export const NAME = "verkstead";

/// What the window's preferences name as its preload, inside the directory the
/// main process was loaded from.
///
/// **`.mjs` rather than `.js`, and it has to be**: a preload ignores the
/// package's `"type": "module"` and reads an extension instead, so an ESM
/// preload is an `.mjs` — which is why the source beside this one is
/// `preload.mts` and why this project compiles it rather than bundling it. What
/// this names and what `pnpm build` emits are the same file, and
/// `tests/bridge.test.ts` is what keeps them the same file.
export const PRELOAD = "preload.mjs";

/// The channel **the settings as they stand** are asked for on.
export const ASKED = "verkstead:settings";

/// The channel a **set** is sent on, carrying what is to change and answering
/// with the settings in force after it — which is what the page draws, so that
/// a refused set is a control that goes back where it was.
export const SET = "verkstead:set";

/// The channel **View Logs** is asked on, which is the tray item's own act
/// reached the other way (ADR-0020).
export const LOGS = "verkstead:logs";

/// The channel **how Launch on Startup stands** is asked for on.
///
/// Apart from [`ASKED`], because it is not one of the app's settings and is
/// deliberately not kept beside them: the platform's own registration is the
/// state (Set 846 Q9a), so this is a question put to the platform rather than a
/// line read out of a file.
export const STARTUP = "verkstead:startup";

/// And the channel a tick of it is sent on, answering with how it stands
/// afterwards — the same shape a set of the settings takes, and for the same
/// reason: what moves the box is the answer rather than the press.
export const REGISTER = "verkstead:register";

/// What `window.verkstead` is, where there is one.
///
/// Five acts and one value, which is the whole of what the page needs: what
/// machine this is, the settings read and written, the log file opened, and the
/// startup registration read and written. Everything asynchronous, because
/// everything but the platform is a question for the process on the other side
/// of the bridge.
export interface Bridge {
  /// Which platform the app is running on — `process.platform`, read in the
  /// preload where there is a process to read it from.
  ///
  /// A value rather than a call: it cannot change while the window is open, and
  /// the page draws different controls for a Mac than for the other two
  /// (ADR-0020), so a page that had to await it would draw the wrong thing
  /// first.
  readonly platform: NodeJS.Platform;

  /// The settings as they stand on this machine.
  settings(): Promise<Settings>;

  /// Change what is named and leave the rest, and answer with the settings in
  /// force afterwards — the set having been enacted in this run rather than
  /// kept for the next launch.
  ///
  /// A set the app does not understand changes nothing and answers with the
  /// settings unchanged, so the control it came from returns to where it was.
  set(changed: Partial<Settings>): Promise<Settings>;

  /// Open this run's log file, or say there is none — the same act the tray's
  /// **View Logs** performs, and it is drawn on the page whatever the tray
  /// setting says.
  logs(): Promise<void>;

  /// How **Launch on Startup** stands on this machine, read from the platform's
  /// own registration rather than from anything kept beside it.
  startup(): Promise<Registration>;

  /// Register or unregister, answering with how it stands afterwards.
  ///
  /// A registration the platform refused answers with what is true and the
  /// reason beside it, so the box goes back where it was and the human is told
  /// why — see [`Registration`](./startup.js).
  register(on: boolean): Promise<Registration>;
}
