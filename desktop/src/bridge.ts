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
//! **What is in this file is what crosses**: the name, the six channels, and
//! the shape. It holds no behaviour and touches no disk on purpose — the
//! preload imports it, so everything it imports is loaded inside the window,
//! and what the app *does* about each of them is `main.ts`'s while what they
//! *mean* is elsewhere: a set is [`changed`](./settings.js)'s, the startup
//! registration is [`startup.ts`](./startup.js)'s, and the head's colours and
//! height are [`decorations.ts`](./decorations.js)'s.
//!
//! **And five of the six go one way while the last goes the other.** Everything
//! the page *asks* is an `invoke` and comes back answered; the head is something
//! the page *says* — the app cannot know what the viewer resolved its tokens to,
//! and there is nothing to answer once it has been told. See [`HEAD`].

import type { Head } from "./decorations.js";
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

/// And the channel the **head's colours and height** are pushed on — the one
/// thing on this bridge the page says rather than asks.
///
/// **Which it has to be the one to say** (ADR-0020, Set 847 Q11b): the app's
/// window has no title bar, so the strip the platform draws its controls on is
/// the app's to paint — and what colour to paint it is something only the page
/// knows, the viewer having a light scheme and a dark one and the head being
/// drawn on the paper of whichever is in force. Two fixed colours here were
/// rejected. The two measurements are the page's for the same kind of reason: the
/// band and the row inside it are written in rem, and what a rem is on that
/// machine is the browser's answer rather than this file's.
///
/// **And on a Mac it is where the lights go rather than what the strip is
/// painted** (Set 889 Q4a): there is no overlay there, and the row the page says
/// it drew is what the traffic lights are moved into the middle of. One push, and
/// the platform decides which of the two it means.
///
/// **A push rather than a question.** There is nothing for the app to answer —
/// the overlay is recoloured, or the lights are moved — and a page that awaited an
/// acknowledgement would be awaiting one at every flip of the scheme. What crosses
/// is [`Head`](./decorations.js).
export const HEAD = "verkstead:head";

/// What `window.verkstead` is, where there is one.
///
/// Six acts and one value, which is the whole of what the page needs: what
/// machine this is, the settings read and written, the log file opened, the
/// startup registration read and written, and the head's colours and height
/// said. Everything the page *asks* is asynchronous, because everything but the
/// platform is a question for the process on the other side of the bridge; the
/// one thing it *says* answers with nothing at all.
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

  /// Say what the head is drawn in, how tall its band stands and where the row
  /// inside it is, so that the controls overlay is the same strip of paper the
  /// head beneath it is — and so that a Mac's traffic lights stand in that row.
  ///
  /// Pushed when the page loads and again at every flip of the colour scheme,
  /// which is the whole of what moves either colour. Nothing comes back: a
  /// platform with an overlay recolours it, a Mac moves its buttons, and neither
  /// is news the page can act on — see [`HEAD`].
  head(worn: Head): void;
}
