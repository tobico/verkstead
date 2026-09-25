//! Which navigations are the window's own, and which are elsewhere's.
//!
//! **The window never navigates away from localhost** (ADR-0020). The workbench
//! is full of links that lead off it — a repository, a pull request, a piece of
//! documentation — and a window that followed one would be a browser: no
//! address bar, no tabs, no back button that anybody can see, and the workbench
//! gone from the one window there is. So every navigation is asked about here
//! first, and anything that is not the server this app started is handed to the
//! browser the human already uses, with the window left exactly where it was.
//!
//! **A new window is the same answer rather than a second window.** A link with
//! `target="_blank"`, or a `window.open` from the page, is a request for a
//! second Electron window, and this app has one window on purpose. What the
//! page was asking for is *that page, somewhere else*, and somewhere else is
//! the browser.
//!
//! **And only what a browser should be handed is handed over.** The platform's
//! opener starts whatever is registered for a scheme, so passing it everything
//! that is not ours would be handing the page a way to start programs on this
//! machine by writing a link. Two schemes go out — the two a browser is for —
//! and everything else is refused: the navigation does not happen and nothing
//! is started.

/// Where a navigation goes.
export type Where =
  /// Stays in this window: the server this app started, which is the only thing
  /// the window is a window onto.
  | "window"
  /// Out to the browser the human already has, with the window left where it is.
  | "browser"
  /// Nowhere at all. Not the window's and not something to start a program
  /// over — see the note about the platform's opener at the top of this file.
  | "nowhere";

/// The schemes a browser is for, and so the only ones ever handed to one.
///
/// Not `mailto:`, not `file:`, not a scheme some other program on this machine
/// has registered for itself: those are the platform's opener asked to start
/// something, which is a different act from opening a page.
const HANDED = new Set(["http:", "https:"]);

/// This machine, however a URL spells it.
///
/// Three spellings of the one host, and the window may meet any of them: the
/// app loads `127.0.0.1` but the workbench's own pages, and anything a human
/// has bookmarked, can as easily say `localhost`. `URL` normalises the rest —
/// the case, and the short forms of the IPv4 address — so what is left to list
/// is the names.
const LOOPBACK = new Set(["127.0.0.1", "localhost", "[::1]"]);

/// Where a navigation to `target` goes, for a window loaded on `origin`.
///
/// The window's own is the one server: this machine, on the scheme and port the
/// app is serving, however the host is spelled. A different port on this same
/// machine is a different program and goes to the browser like anything else —
/// there is one Verkstead here and it is on the one address.
export function where(target: string, origin: string): Where {
  const asked = parse(target);

  // Anything `URL` cannot read is nothing to navigate to and nothing to hand
  // over either.
  if (asked === undefined) {
    return "nowhere";
  }

  const home = parse(origin);

  if (
    home !== undefined &&
    asked.protocol === home.protocol &&
    asked.port === home.port &&
    LOOPBACK.has(asked.hostname)
  ) {
    return "window";
  }

  return HANDED.has(asked.protocol) ? "browser" : "nowhere";
}

/// `target` read as a URL, or `undefined` where it is not one.
function parse(target: string): URL | undefined {
  try {
    return new URL(target);
  } catch {
    return undefined;
  }
}
