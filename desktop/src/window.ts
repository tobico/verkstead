//! The one window, opened on the login link and kept a window onto Verkstead.
//!
//! The second file at the edge, and the first one that draws anything: it holds
//! a `BrowserWindow`, so vitest cannot run it and the lint wall in
//! `eslint.config.js` names it. Everything it decides is somewhere else — where
//! the workbench is, what the key is, how a link is built, which navigations are
//! its own, where it can open — and what is left here is the window and the
//! events it answers.
//!
//! **Loaded exactly as it is served.** Nothing about the viewer changes to draw
//! inside the app and nothing on the wire changes: this is the same document a
//! browser on this machine gets, over the same loopback origin. What is
//! different is the window rather than the document — it carries the preload in
//! [`bridge.ts`](./bridge.js), and the page reads that being there as *this is
//! the app*. A browser gets no preload and so no Desktop page, which is the
//! whole mechanism by which a phone never sees one.
//!
//! **A 401 on the window's own frame is the key having been reset from the
//! phone.** **Reset key** at the foot of Remote Access re-issues the secret, and
//! everything holding the old one meets a 401 on its next request. So the frame's
//! own navigation response is watched — not a subresource's, which is a different
//! failure with a different answer — the file is read again and the link is
//! loaded again. Once per navigation, so a key that is genuinely unreadable is a
//! window sitting on a refusal rather than a loop.
//!
//! **And it never navigates away from localhost.** Every link that leads off the
//! workbench goes to the browser the human already has, and a page asking for a
//! second window gets the same answer rather than a second window — the rule is
//! [`elsewhere.ts`](./elsewhere.js)'s and the two events that ask it are here.
//! Where the window opens is [`bounds.ts`](./bounds.js)'s, out of a file of the
//! app's own, and the menu whose bar this hides is [`menu.ts`](./menu.js)'s.
//!
//! **And closing it is a choice rather than a quit.** What a press of the close
//! button comes to is [`closing.ts`](./closing.js)'s answer, out of the app's
//! own settings and the platform — hide the window, ask first, or let the close
//! happen — and what is here is the three of them enacted: the cancel that
//! makes hiding possible at all, the native warning, and the bounds still being
//! written down by a close that never completes.
//!
//! **And a login start comes up with it off the screen.** The one thing
//! [`Workbench.hidden`] decides: the window is made, loads and remembers where
//! it is exactly as ever, and what it does not do is arrive in front of whatever
//! the human is doing at the moment they log in — the reading `--no-open` made
//! of a login for the tray app, and [`hidden`](./startup.js)'s to make.
//!
//! **And it has no title bar.** What stands where one would have been is the
//! platform's own — the controls overlay at the top-right on Windows and Linux,
//! the traffic lights at the top-left on a Mac, inset to the row a head's title
//! stands in — and the options that ask for that are
//! [`decorations.ts`](./decorations.js)'s, for the reason every other value here
//! is somewhere else. Where they go afterwards is the page's, over the bridge.

import { BrowserWindow, dialog, screen, shell } from "electron";

import {
  asGiven,
  type Drift,
  drifted,
  fit,
  type Placement,
  remember,
  remembered,
  STILL,
} from "./bounds.js";
import { CANCEL, type Closing, QUIT, WARNING } from "./closing.js";
import { DECORATIONS } from "./decorations.js";
import { where } from "./elsewhere.js";
import { link } from "./key.js";
import { why } from "./loading.js";
import { say } from "./log.js";
import { hand, type Opening } from "./opening.js";

/// What the window opens at on a machine that has not told it otherwise — a
/// desktop-sized workbench rather than a phone-sized one. Once it has been moved
/// or resized this is never read again: what the human did is what the next run
/// opens at.
const SIZE = { width: 1280, height: 860 };

/// How long a drag or a resize is left to finish before where it ended up is
/// written down. Dragging a window emits a move for every frame of it, and a
/// state file rewritten sixty times a second is a disk kept busy for no reason.
const SETTLED = 400;

/// What the gate answers anything that has not shown the current key.
const REFUSED = 401;

/// What opening the window needs, none of which it works out for itself.
export interface Workbench {
  /// The origin the window loads, which is the one the sidecar answers on — and
  /// the one navigation is measured against.
  origin: string;

  /// The **Workbench Key** as it is on disk *now*, or `undefined` where there is
  /// none to read. Asked again at every load rather than once: that is what
  /// makes a reset from the phone a reload rather than a restart.
  secret: () => string | undefined;

  /// The file this window's size and position are kept in, under the app's own
  /// user data — a fact about this machine rather than about this Verkstead.
  state: string;

  /// The preload script this window is given, which is the whole of what makes
  /// the page inside the app different from the same page in a browser — see
  /// [`bridge.ts`](./bridge.js). An absolute path, because this is a file the
  /// app ships beside itself rather than anything the page names.
  preload: string;

  /// Whether this launch comes up with no window on the screen — a login start
  /// while there is an icon in the tray, which is [`hidden`](./startup.js)'s
  /// answer.
  ///
  /// The window is made either way, and everything about it is as it always is:
  /// it loads, it remembers where it is, and Open on the tray is what brings it
  /// on. What is different is only that nothing arrives over whatever the human
  /// is doing at the moment they log in.
  hidden: boolean;

  /// What this press of the close button means — [`closing`](./closing.js)'s
  /// answer, asked at the moment of the press rather than once at startup.
  ///
  /// A function rather than a value because what is behind it moves while the
  /// window is open: the settings file is read then, so a position set on the
  /// Desktop page is in force without a restart. It is also where the app says
  /// *this close is a quit I am already performing* — the tray's Quit and Cmd+Q
  /// reach this window as a close on their way out, and neither of them is the
  /// close button.
  closing: () => Closing;

  /// How this machine hands a url to a browser — [`opening`](./opening.js)'s
  /// answer, and `undefined` on the two platforms where that is `shell`'s own
  /// job. A link that leaves the workbench is the other of the two things this
  /// app opens, and it is opened the same way the log file is.
  by: Opening | undefined;
}

/// Open the window on the workbench, logged in and where it was left.
export function open(workbench: Workbench): BrowserWindow {
  // The displays as they are *now*, which is the whole question: the file was
  // written on whatever was plugged in last time.
  const place = fit(remembered(workbench.state), screen.getAllDisplays(), SIZE);

  const window = new BrowserWindow({
    ...place,
    // What the window is called until the document says, which is the product
    // rather than the package this is built from.
    title: "Verkstead",

    // Off the screen where this is a login start with a tray to be reached by,
    // and on it every other time. A hidden window rather than no window: it is
    // loading the workbench behind the icon, so Open is a window that is already
    // there — which is exactly what a close that hides leaves behind.
    show: !workbench.hidden,

    // The menu itself is set — it is what registers copy, paste, zoom, reload
    // and the developer tools — and what is hidden is the bar it would be drawn
    // in. Both, because the option alone leaves the bar drawn until something
    // hides it and the call alone leaves Alt showing it permanently. On a Mac
    // neither does anything: its menu is the strip at the top of the screen.
    autoHideMenuBar: true,

    // No title bar, and the platform's own controls where it was — the style,
    // the overlay and the traffic lights' opening point together, out of
    // [`decorations.ts`](./decorations.js). All of it is where the window
    // starts; the page is what it ends up wearing.
    ...DECORATIONS,

    webPreferences: {
      // The bridge, and the whole of what the app adds to the document the
      // browser gets. A file inside the app rather than a path from anywhere
      // else.
      preload: workbench.preload,

      // **And Chromium's own sandbox off, which the preload above is what
      // decides.** An ESM preload is loaded only in a window that has it off —
      // and a sandboxed preload, which is the alternative, may require nothing
      // but `electron` itself, so it could not import the shape it exposes in a
      // project that compiles rather than bundles (ADR-0020).
      //
      // What is not given up is the pair that matters to a page: node
      // integration stays off, so nothing in the document has a `require`, and
      // `contextIsolation` stays on, so the preload's own world is not the
      // page's. Which leaves a renderer that loads one document — this
      // machine's own Verkstead, on loopback, with every navigation off it
      // handed to the browser by `bound` below.
      sandbox: false,
    },
  });
  window.setMenuBarVisibility(false);

  keeping(window, workbench.state, place);
  policy(window, workbench.closing);
  bound(window, workbench.origin, workbench.by);

  // Whether the load now on its way is already an answer to a refusal. Set when
  // one is made and cleared by any navigation that was not refused, so a
  // refusal has exactly one retry behind it however many times it is met.
  let recovering = false;

  const load = (): void => {
    const secret = workbench.secret();

    if (secret === undefined) {
      // The bare address, which the gate refuses — and that refusal is what
      // reads the file again, moments later. The one mechanism for both the key
      // that has been reset and the key that is not written yet.
      say(`no workbench key to read yet, so the window opens on ${workbench.origin} bare`);
    }

    const target = secret === undefined ? workbench.origin : link(workbench.origin, secret);

    // The origin rather than the link, here and everywhere: these lines go to a
    // file a menu item opens on somebody's desk, and a key written there is a
    // login for anybody reading over a shoulder (ADR-0015). Which the failure
    // has to be worded for as well as the success — Electron puts the URL it
    // was handed into a rejected `loadURL`, so the reason is read off the
    // error's own fields by [`why`](./loading.js) rather than stringified.
    window.webContents.loadURL(target).catch((trouble: unknown) => {
      say(`the window could not load ${workbench.origin} — ${why(trouble)}`);
    });
  };

  // A main frame navigation and its response code. `did-navigate` is the main
  // frame's alone — a subresource's 401 is something else, answered somewhere
  // else — and a navigation that was no HTTP request at all reports -1, which is
  // not a refusal either.
  window.webContents.on("did-navigate", (_event, _url, code) => {
    if (code !== REFUSED) {
      recovering = false;
      return;
    }

    if (recovering) {
      say("the workbench refused the key that was just read — the window is on a refusal");
      return;
    }

    recovering = true;
    say("the workbench refused the window's key, so it is read again");
    load();
  });

  say(`the window is opening on ${workbench.origin}`);
  load();

  return window;
}

/// Keep where the window is, for the next run to open at.
///
/// **The normal bounds rather than the current ones**: a maximised or
/// full-screen window is the size of the screen, and a run that remembered that
/// would open maximised-looking and un-maximisable. What is kept is where the
/// window would go back to.
///
/// **And measured against what was asked for**, which is [`drifted`]'s reason
/// for existing: a desktop that frames a window reports it back a few pixels
/// from where it was put, and a run that wrote that down would open a window
/// that grew at every launch. What is written is what this run asked for, plus
/// whatever the human did to it.
function keeping(window: BrowserWindow, state: string, place: Placement): void {
  let settling: ReturnType<typeof setTimeout> | undefined;

  // Nothing yet, and on most desktops nothing ever — the measurement below is
  // what fills it in, once the window has been framed.
  let drift: Drift = STILL;

  // Measured a moment after the window is on the screen rather than at once:
  // what is being measured is the desktop's answer, and the answer is what it
  // has done to the window by the time it has finished drawing it. The same
  // moment a drag is given to settle in, for the same reason.
  //
  // **After it is shown, which a login start is not.** A window that has not
  // been drawn has not been framed either, so a hidden start measured at once
  // would read a drift of nothing and write that down — and the next run would
  // open a window a frame's worth larger than the one the human left.
  const measure = (): void => {
    const framed = setTimeout(() => {
      drift = drifted(place, window.getNormalBounds());
    }, SETTLED);
    framed.unref();
  };

  if (window.isVisible()) {
    measure();
  } else {
    window.once("show", measure);
  }

  const now = (): void => {
    settling = undefined;
    remember(state, asGiven(window.getNormalBounds(), drift));
  };

  const soon = (): void => {
    if (settling !== undefined) {
      clearTimeout(settling);
    }
    settling = setTimeout(now, SETTLED);
    // Nothing waits on this: a pending write is not a reason for the process to
    // still be here.
    settling.unref();
  };

  window.on("resize", soon);
  window.on("move", soon);

  // And once more as it goes, whatever the drag that was still settling had got
  // to — this is the write that matters, the others being insurance against the
  // run that ends without a close.
  window.on("close", () => {
    if (settling !== undefined) {
      clearTimeout(settling);
    }
    now();
  });
}

/// Do what the close policy says about a press of the close button.
///
/// **Registered after [`keeping`] on purpose.** A close that hides has to leave
/// the window's place written down — an app that always keeps running would
/// otherwise be an app that never remembers its window again — and both
/// handlers run whether or not this one cancels the close, so which of them
/// ran first is the whole of what decides it.
///
/// **And cancelling is how hiding is done at all.** `close` is the window on
/// its way out, and the only way to stop it is to say so before the handler
/// returns — which is why the warning is a synchronous dialog rather than an
/// awaited one: a `preventDefault` arriving a tick later would arrive at a
/// window that had already gone.
function policy(window: BrowserWindow, closing: () => Closing): void {
  window.on("close", (event) => {
    const act = closing();

    if (act === "quit") {
      // Which is also every quit the app performs for itself: the tray's Quit
      // and Cmd+Q reach the window as a close, and neither is asked about.
      return;
    }

    if (act === "hide") {
      event.preventDefault();
      window.hide();
      say("the window is closed, and Verkstead goes on running — the tray is the way back");
      return;
    }

    const answered = dialog.showMessageBoxSync(window, {
      type: "warning",
      title: "Verkstead",
      message: WARNING.message,
      detail: WARNING.detail,
      buttons: [...WARNING.buttons],

      // The way on is what the dialog opens on, and the way out is what Escape
      // and the dialog's own close button come to. Both said, because a
      // platform that draws no default still has to answer a dismissal.
      defaultId: QUIT,
      cancelId: CANCEL,

      // Buttons side by side rather than drawn as links, which is what a
      // question with an action and a way out is on every platform this runs
      // on.
      noLink: true,
    });

    if (answered === CANCEL) {
      event.preventDefault();
      say("the warning was answered with Cancel, so the window stays");
      return;
    }

    say("the warning was answered with Quit, so Verkstead goes");
  });
}

/// Keep the window on the one origin, and hand everything else over.
///
/// Two events, because a page has two ways of going somewhere: following a link
/// in this frame, and asking for a window to follow it in. The answer is the
/// same for both — the workbench stays on the screen and the browser gets the
/// page — and a new window is never made either way.
function bound(window: BrowserWindow, origin: string, by: Opening | undefined): void {
  window.webContents.setWindowOpenHandler(({ url }) => {
    away(url, origin, by);
    return { action: "deny" };
  });

  window.webContents.on("will-navigate", (event, url) => {
    if (where(url, origin) === "window") {
      return;
    }

    // Stopped before it starts: the window is still showing the workbench when
    // the browser comes up in front of it.
    event.preventDefault();
    away(url, origin, by);
  });
}

/// Hand `url` to the browser, or to nothing at all.
///
/// Not awaited: what starts is somebody else's program, and a browser that
/// takes ten seconds to come up is not something the app should be waiting on —
/// the same reading `crates/desktop/src/opener.rs` made of it.
function away(url: string, origin: string, by: Opening | undefined): void {
  if (where(url, origin) !== "browser") {
    say(`the window was asked to open ${url}, which is nothing a browser is for, so it does not`);
    return;
  }

  say(`${url} is not this Verkstead, so it goes to the browser`);

  // Ours to open where this machine has said so, for the reason **View Logs** is
  // — see [`opening`](./opening.js): a browser started off this process's own
  // environment is a browser loading the bundle's libraries.
  if (by !== undefined) {
    hand(by, url, say);
    return;
  }

  shell.openExternal(url).catch((trouble: unknown) => {
    say(`the browser could not be given ${url} — ${String(trouble)}`);
  });
}

/// Bring the window forward, which is what a second launch of the app comes to.
///
/// Electron's single-instance lock hands the second launch's argument over to
/// the first and ends it, so this runs in the app that is already here. Three
/// steps rather than a `focus`, because the window this reaches may be
/// minimised, may be hidden behind everything, and may — from stage 03, where
/// closing can mean keep running — not be on the screen at all.
export function forward(window: BrowserWindow): void {
  if (window.isMinimized()) {
    window.restore();
  }
  window.show();
  window.focus();
}
