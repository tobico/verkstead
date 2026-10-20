//! The preload: the one script that runs inside the window, and the whole of
//! what the page can reach the app by.
//!
//! The fourth file at the edge, and the only one that is not the main process:
//! it imports `electron` for `contextBridge` and `ipcRenderer`, so the lint
//! wall in `eslint.config.js` names it. What it puts on the window is
//! [`bridge.ts`](./bridge.js)'s shape and nothing else — one value and nine
//! calls, each of them a message to the main process or a listener for one,
//! which is where the file is read, the icon is raised, the log is opened, the
//! platform is asked about its startup registration and its notifications, the overlay is recoloured,
//! the Go menu is told what it can go to, and a clicked notification's path is
//! heard.
//!
//! **Nothing is exposed but the ten.** `contextBridge` is what makes that
//! true: the page runs in a world of its own with no Node in it, and what
//! crosses is this object rather than a process, a filesystem or an
//! `ipcRenderer`. A set arriving on the other side is checked against the shape
//! before it reaches the file all the same — see
//! [`changed`](./settings.js) — because a bridge that trusted its renderer
//! would only have moved the question.
//!
//! **An ESM preload, which is two facts about this file.** A preload ignores
//! the package's `"type": "module"` and reads the extension instead, so this is
//! a `.mts` compiled to the `.mjs` the window names; and Electron loads an ESM
//! preload only in a window whose `sandbox` is off, which is why `window.ts`
//! turns it off and says so there. The alternative was a sandboxed CommonJS
//! preload, and it cannot be one here: a sandboxed preload may require nothing
//! but `electron` itself, so it could not import the shape above — and this
//! project compiles rather than bundles (ADR-0020), so there is nothing to roll
//! the two files into one. What is kept instead is `contextIsolation`, which is
//! what stands between the page and this script, and a window that never leaves
//! the loopback origin.

import { contextBridge, ipcRenderer } from "electron";

import {
  ASKED,
  type Bridge,
  HEAD,
  LOGS,
  NAME,
  NOTIFIABLE,
  OPEN,
  REGISTER,
  SET,
  STARTUP,
  WHERE,
} from "./bridge.js";
import type { Settings } from "./settings.js";
import type { Registration } from "./startup.js";

/// What `window.verkstead` is inside the app.
///
/// Every call but the last two is an `invoke`, which is a question with an
/// answer: a set is answered with the settings in force after it, so what moves
/// the control on the page is what came back rather than what was pressed.
///
/// The last two are a `send`, which is a statement: the head's colours and height,
/// and what the open Conversation can go to, are things the page knows and the
/// app does not, and there is nothing to answer once they have been said — see
/// [`HEAD`](./bridge.js) and [`WHERE`](./bridge.js).
///
/// And the listener is the other way round: the app tells, and what the page
/// hears is the path alone rather than Electron's event, which carries the
/// sender and is nothing the page should hold. Starting and stopping are said
/// back on the same channel, which is how the app knows there is a page to tell
/// — see [`OPEN`](./bridge.js).
const bridge: Bridge = {
  // Read here rather than asked for, because here is where there is a process
  // to read it from and because the page draws different controls for a Mac.
  platform: process.platform,

  settings: () => ipcRenderer.invoke(ASKED) as Promise<Settings>,
  set: (changed) => ipcRenderer.invoke(SET, changed) as Promise<Settings>,
  logs: () => ipcRenderer.invoke(LOGS) as Promise<void>,

  startup: () => ipcRenderer.invoke(STARTUP) as Promise<Registration>,
  register: (on) => ipcRenderer.invoke(REGISTER, on) as Promise<Registration>,

  notifiable: () => ipcRenderer.invoke(NOTIFIABLE) as Promise<boolean>,

  head: (worn) => ipcRenderer.send(HEAD, worn),
  where: (open) => ipcRenderer.send(WHERE, open),

  opening: (heard) => {
    const told = (_event: unknown, path: unknown) => {
      if (typeof path === "string") {
        heard(path);
      }
    };

    ipcRenderer.on(OPEN, told);
    ipcRenderer.send(OPEN, true);

    return () => {
      ipcRenderer.removeListener(OPEN, told);
      ipcRenderer.send(OPEN, false);
    };
  },
};

contextBridge.exposeInMainWorld(NAME, bridge);
