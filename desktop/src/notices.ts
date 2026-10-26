//! The notices stream, held for the app's whole run.
//!
//! **Electron ships without Google's push client**, so the Web Push a phone is
//! told through is refused inside this window however many checks pass on the
//! loopback. What the app does instead is ask the sidecar directly: `GET
//! /api/ui/push/notices` is a server-sent-events stream of every notice the
//! server pushes to its phones, as the same JSON a service worker is handed —
//! see `crates/server/src/push.rs`, which is the other end of this file
//! (ADR-0020, *Amended: notifications are the app's own*).
//!
//! **Held rather than polled, and lost rather than replayed.** The stream is
//! opened once the server is healthy and opened again whenever it drops, after
//! a short wait that grows to a bound and no further. A notice that arrives
//! while it is down is gone, as a push is when its service is: nothing on the
//! server queues for a reader that is not there, and a notification about
//! something that happened minutes ago is the workbench's to show rather than
//! the desktop's.
//!
//! **Logged in with the Workbench Key, read afresh at every opening** — the
//! reading [`keyIn`](./key.js) makes for the window's link, and for its reason:
//! **Reset key** on the phone re-issues the secret while the app is running, so a
//! 401 is the key having changed and the answer is to read the file again. The
//! key goes in the cookie the gate reads rather than on the query, which the gate
//! answers with a redirect to set that same cookie.
//!
//! **What is pure is apart from the holding**: [`Frames`] reads the stream's
//! bytes into notices and [`showing`] says whether one is worth a notification,
//! and both are values vitest runs. [`hold`] is the loop, over Node's own
//! `fetch` and handed everything else — so a real server stands in for the
//! sidecar in the tests, and drawing a notification is the edge's, in
//! `main.ts`.

/// What the stream calls the events it carries a notice in — the server's
/// `Event::event("notice")`.
const EVENT = "notice";

/// What the gate reads a login out of — the server's `key::COOKIE_NAME`.
export const COOKIE = "workbench_key";

/// One notice: what the phones are told, as the server says it.
export interface Notice {
  /// The page a click opens, relative to the workbench.
  path: string;

  /// The sentence, which is the notification's title.
  title: string;

  /// Which repository it is about, where there is one — the body.
  project?: string;
}

/// Read one event's data as a notice, or `undefined` where it is not one.
///
/// The stream is the app's own sidecar, but a parse that trusted the shape would
/// be a notification titled `undefined` the day the two disagree.
export function notice(data: string): Notice | undefined {
  let parsed: unknown;

  try {
    parsed = JSON.parse(data);
  } catch {
    return undefined;
  }

  if (typeof parsed !== "object" || parsed === null) {
    return undefined;
  }

  const { path, title, project } = parsed as Record<string, unknown>;

  if (typeof path !== "string" || typeof title !== "string") {
    return undefined;
  }

  if (project !== undefined && typeof project !== "string") {
    return undefined;
  }

  return project === undefined ? { path, title } : { path, title, project };
}

/// The stream's bytes, read into notices as whole frames arrive.
///
/// **Frames are not the chunks they arrive in**: a chunk may end halfway through
/// a line, or hold three frames at once, so what has not made a whole frame yet
/// is kept for the next. The rest is the event-stream format as far as this
/// stream uses it — a blank line ends a frame, a line opening with a colon is a
/// keep-alive comment, `event:` names the frame and `data:` lines carry it — and
/// a frame that is not a notice is skipped rather than shown.
export class Frames {
  private buffered = "";

  /// Take `chunk` and hand back the notices it finished.
  push(chunk: string): Notice[] {
    let text = this.buffered + chunk;

    // A CR as the very last character may be the first half of a CRLF still to
    // come, which read now would be a blank line ending a frame early — so it
    // waits for the chunk that says which it was.
    const held = text.endsWith("\r") ? "\r" : "";
    if (held !== "") {
      text = text.slice(0, -1);
    }

    // Every line ending the format allows — CRLF, LF and a lone CR — read as one.
    const lines = text.split(/\r\n|\r|\n/);

    const finished: Notice[] = [];
    let start = 0;

    for (let at = 0; at < lines.length - 1; at++) {
      if (lines[at] !== "") {
        continue;
      }

      const shown = framed(lines.slice(start, at));
      if (shown !== undefined) {
        finished.push(shown);
      }
      start = at + 1;
    }

    // Whatever has not been ended by a blank line yet, the unfinished last line
    // among it, waits for the chunk after this one.
    this.buffered = lines.slice(start).join("\n") + held;

    return finished;
  }
}

/// One frame's lines as a notice, where it is one.
function framed(lines: string[]): Notice | undefined {
  let event = "message";
  const data: string[] = [];

  for (const line of lines) {
    if (line.startsWith(":")) {
      continue;
    }

    const colon = line.indexOf(":");
    const field = colon === -1 ? line : line.slice(0, colon);
    let value = colon === -1 ? "" : line.slice(colon + 1);
    if (value.startsWith(" ")) {
      value = value.slice(1);
    }

    if (field === "event") {
      event = value;
    } else if (field === "data") {
      data.push(value);
    }
  }

  if (event !== EVENT || data.length === 0) {
    return undefined;
  }

  return notice(data.join("\n"));
}

/// The window, as far as whether a notice is worth showing goes.
export interface Seen {
  /// On the screen at all: not hidden to the tray, and not a login start that
  /// never showed it.
  visible: boolean;

  /// Down in the taskbar or the Dock.
  minimized: boolean;

  /// In front, taking the keyboard.
  focused: boolean;
}

/// Whether a notice is shown, and why not where it is not.
export type Showing = { show: true } | { show: false; why: string };

/// Whether a notice is worth a notification, given the window and whether the
/// app is to show them at all.
///
/// **Only while the window is not in front.** A focused window already draws the
/// news — the Timeline moves, the Set lands, the Nudge sounds — and a
/// notification over it is the same news told twice. Hidden to the tray,
/// minimised, behind another app or never shown on a login start are all a
/// window the human is not looking at, so all four are shown.
///
/// `on` is the app's own setting for notifications, which is a switch the page
/// moves; off withholds everything whatever the window is doing.
export function showing(seen: Seen, on: boolean): Showing {
  if (!on) {
    return { show: false, why: "notifications are switched off" };
  }

  if (seen.visible && !seen.minimized && seen.focused) {
    return { show: false, why: "the window is in front" };
  }

  return { show: true };
}

/// Where a click on `arrived` opens, as a path on the workbench: the notice's
/// own, or the workbench's root where that is not a path of this origin's.
///
/// **Checked because it becomes a navigation of the app's own window**, and both
/// ways a click goes put it on the window's origin by joining it to the end: a
/// path that did not lead with one slash, or led with two — `//elsewhere`, which
/// a browser reads as another host — is a window walked off the loopback by
/// whatever the stream said. The stream is the app's own sidecar, but a click is
/// a navigation and the window's rule is that it never leaves localhost.
export function opens(arrived: Notice): string {
  const path = arrived.path;

  if (!path.startsWith("/") || path.startsWith("//") || path.includes("\\")) {
    return "/";
  }

  return path;
}

/// How long to wait before opening the stream again, `after` that many failures
/// in a row: short at first, because a drop is usually a moment, and never more
/// than the bound, because a notice is only worth anything while it is news.
export function pause(after: number, waiting: Waiting = {}): number {
  const first = waiting.first ?? FIRST;
  const most = waiting.most ?? MOST;

  return Math.min(most, first * 2 ** Math.max(0, after - 1));
}

/// The first wait after a drop.
const FIRST = 250;

/// The longest wait between two openings.
const MOST = 5_000;

/// What a caller may say about the waiting instead — the tests', which have no
/// interest in seconds of it.
export interface Waiting {
  /// The first wait after a drop, in milliseconds.
  first?: number;

  /// The bound, in milliseconds.
  most?: number;
}

/// What [`hold`] is handed: where the stream is, how to log in to it, what to do
/// with a notice, and what to say.
export interface Holding {
  /// The stream's URL.
  url: string;

  /// The key as it is on disk now, read at every opening.
  secret: () => string | undefined;

  /// A notice, as it arrives.
  heard: (notice: Notice) => void;

  /// A line for the log.
  say: (line: string) => void;

  /// How long to wait between openings.
  waiting?: Waiting;
}

/// Sleep for `ms`, or until `signal` says the holding is over.
function asleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((done) => {
    const woken = setTimeout(done, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(woken);
        done();
      },
      { once: true },
    );
  });
}

/// Hold the stream until `signal` aborts: open it, hand every notice to
/// `heard`, and open it again whenever it drops.
///
/// **Says when it is held**, in a line the release legs read for — the same
/// words at every opening, so that the log of a run that reconnected says so —
/// and says each drop, so that a stream that keeps dropping is visible. A 401 is
/// said as the key having been reset, and the next opening reads it again; a
/// key that changes on disk while the stream is open is the stream opened again
/// at once.
export async function hold(holding: Holding, signal: AbortSignal): Promise<void> {
  let failures = 0;

  while (!signal.aborted) {
    const secret = holding.secret();

    try {
      const answer = await fetch(holding.url, {
        headers: secret === undefined ? {} : { cookie: `${COOKIE}=${secret}` },
        signal,
      });

      if (answer.status === 401) {
        holding.say("the notices stream refused the key — it was reset, so it is read again");
      } else if (!answer.ok || answer.body === null) {
        holding.say(`the notices stream answered ${answer.status}`);
      } else {
        holding.say(`the notices stream is held at ${holding.url}`);
        failures = 0;

        const frames = new Frames();
        const decoder = new TextDecoder();

        let reset = false;

        for await (const chunk of answer.body) {
          for (const arrived of frames.push(decoder.decode(chunk, { stream: true }))) {
            holding.heard(arrived);
          }

          // A stream already open outlives a **Reset key** — the gate is asked
          // once, at the opening — so the file is read again at every chunk,
          // keep-alives among them, and a key that has changed is the stream
          // opened again under the new one rather than held under a secret the
          // human has just revoked.
          if (holding.secret() !== secret) {
            reset = true;
            break;
          }
        }

        if (reset) {
          holding.say("the Workbench Key was reset, so the notices stream is opened again under the new one");
          continue;
        }

        holding.say("the notices stream ended, so it is opened again");
      }
    } catch (trouble) {
      if (signal.aborted) {
        return;
      }
      holding.say(`the notices stream dropped — ${String(trouble)}`);
    }

    failures += 1;
    await asleep(pause(failures, holding.waiting), signal);
  }
}
